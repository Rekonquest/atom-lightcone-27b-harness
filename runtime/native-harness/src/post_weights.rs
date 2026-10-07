//! Direct post-weights Lightcone binding.
//!
//! Ordering contract (the harness's standing architecture directive):
//!
//! ```text
//! Prism weights -> raw logits -> lightcone-direct-post-weights -> token selection
//! ```
//!
//! A verified passive admission exposes admitted nodes. Their labels and aliases
//! become a *bounded, adapter-owned* set of preferred tokens; the filter runs as
//! a registered llama sampler named `lightcone-direct-post-weights` over the raw
//! logits and adds a capped boost to those tokens only, and only when the top-2
//! margin says the step is undecided. With no grounding — an unknown or
//! out-of-cone query, or a pack that admitted nothing — it is exact
//! pass-through: the logits leave byte-identical and selection is unchanged.
//!
//! Lightcone stays passive geometry here. It contributes a vocabulary and a
//! receipt hash; every number that bounds the influence (confidence, margin,
//! scale, cap) is an adapter constant. No verdict, no score, no authorization.
//!
//! Attachment note, deliberate: the reference implementation put this filter in
//! a llama sampler chain followed by greedy. This harness keeps its own frozen
//! selection stage (top-k 20 / top-p 0.95 / base temperature 1.0 / fixed
//! xorshift seed), so the filter is applied through `llama_sampler_apply` and
//! the selection stage is untouched — same boundary position, same math, same
//! bounds, and both arms still share one selection stage.

use crate::ffi::*;
use std::os::raw::{c_char, c_void};
use std::ptr;

/// Bounded adapter-owned token preferences derived from one admission.
#[derive(Clone, Debug)]
pub struct TokenGrounding {
    pub preferred_tokens: Vec<llama_token>,
    pub receipt_hash: String,
    pub confidence: f32,
    pub uncertainty_margin: f32,
    pub bias_scale: f32,
    pub bias_cap: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HookStats {
    pub raw_logit_passes: usize,
    pub fires: usize,
    pub deflections: usize,
}

#[derive(Clone)]
struct HookContext {
    grounding: Option<TokenGrounding>,
    stats: HookStats,
}

/// The registered Lightcone sampler. `passthrough()` carries no grounding and
/// therefore cannot change a single logit.
pub struct LightconeFilter {
    sampler: *mut llama_sampler,
}

impl LightconeFilter {
    pub fn passthrough() -> Result<Self, String> {
        Self::build(None)
    }

    pub fn with_lightcone(grounding: TokenGrounding) -> Result<Self, String> {
        if grounding.preferred_tokens.is_empty() {
            return Err("direct Lightcone binding requires preferred tokens".into());
        }
        if grounding.receipt_hash.len() != 64 {
            return Err("direct Lightcone binding requires a SHA-256 receipt hash".into());
        }
        if !grounding.confidence.is_finite() || !(0.0..=1.0).contains(&grounding.confidence) {
            return Err("grounding confidence must be within [0, 1]".into());
        }
        if !grounding.uncertainty_margin.is_finite() || grounding.uncertainty_margin <= 0.0 {
            return Err("uncertainty margin must be positive".into());
        }
        Self::build(Some(grounding))
    }

    fn build(grounding: Option<TokenGrounding>) -> Result<Self, String> {
        let context = Box::into_raw(Box::new(HookContext {
            grounding,
            stats: HookStats::default(),
        }));
        let sampler = unsafe {
            llama_sampler_init(
                ptr::addr_of!(POST_WEIGHTS_INTERFACE).cast_mut(),
                context.cast::<c_void>(),
            )
        };
        if sampler.is_null() {
            unsafe {
                drop(Box::from_raw(context));
            }
            return Err("direct post-weights sampler allocation failed".into());
        }
        Ok(Self { sampler })
    }

    /// Apply the filter over raw logits, before any token selection.
    pub fn apply(&mut self, array: &mut llama_token_data_array) -> Result<(), String> {
        if self.sampler.is_null() {
            return Err("direct post-weights sampler is absent".into());
        }
        unsafe {
            llama_sampler_apply(self.sampler, array);
        }
        Ok(())
    }

    pub fn stats(&self) -> HookStats {
        if self.sampler.is_null() {
            return HookStats::default();
        }
        let context = unsafe { (*self.sampler).ctx.cast::<HookContext>() };
        if context.is_null() {
            HookStats::default()
        } else {
            unsafe { (*context).stats }
        }
    }
}

impl Drop for LightconeFilter {
    fn drop(&mut self) {
        unsafe {
            llama_sampler_free(self.sampler);
        }
    }
}

static POST_WEIGHTS_INTERFACE: llama_sampler_i = llama_sampler_i {
    name: Some(hook_name),
    accept: None,
    apply: Some(hook_apply),
    reset: None,
    clone: Some(hook_clone),
    free: Some(hook_free),
    backend_init: None,
    backend_accept: None,
    backend_apply: None,
    backend_set_input: None,
    backend_reset: None,
    copy_state: None,
};

unsafe extern "C" fn hook_name(_: *const llama_sampler) -> *const c_char {
    c"lightcone-direct-post-weights".as_ptr()
}

unsafe extern "C" fn hook_apply(sampler: *mut llama_sampler, candidates: *mut llama_token_data_array) {
    if sampler.is_null() || candidates.is_null() {
        return;
    }
    let context = unsafe { (*sampler).ctx.cast::<HookContext>() };
    if context.is_null() {
        return;
    }
    let array = unsafe { &mut *candidates };
    if array.data.is_null() {
        return;
    }
    let data = unsafe { std::slice::from_raw_parts_mut(array.data, array.size) };
    apply_direct(unsafe { &mut *context }, data, array);
}

unsafe extern "C" fn hook_clone(sampler: *const llama_sampler) -> *mut llama_sampler {
    if sampler.is_null() {
        return ptr::null_mut();
    }
    let context = unsafe { (*sampler).ctx.cast::<HookContext>() };
    if context.is_null() {
        return ptr::null_mut();
    }
    let cloned = Box::into_raw(Box::new(unsafe { (*context).clone() }));
    unsafe {
        llama_sampler_init(
            ptr::addr_of!(POST_WEIGHTS_INTERFACE).cast_mut(),
            cloned.cast::<c_void>(),
        )
    }
}

unsafe extern "C" fn hook_free(sampler: *mut llama_sampler) {
    if sampler.is_null() {
        return;
    }
    let context = unsafe { (*sampler).ctx.cast::<HookContext>() };
    if !context.is_null() {
        unsafe {
            drop(Box::from_raw(context));
            (*sampler).ctx = ptr::null_mut();
        }
    }
}

/// The reference filter, verbatim in substance: count the pass, return exactly
/// when there is no grounding, apply the margin gate, then boost preferred
/// tokens by a capped amount and note whether the argmax moved.
fn apply_direct(
    context: &mut HookContext,
    candidates: &mut [llama_token_data],
    array: &mut llama_token_data_array,
) {
    context.stats.raw_logit_passes += 1;
    let Some(grounding) = &context.grounding else {
        return;
    };
    let Some((before, top, second)) = top_two(candidates) else {
        return;
    };
    let margin = top - second;
    if margin >= grounding.uncertainty_margin {
        return;
    }
    context.stats.fires += 1;
    let urgency = 1.0 - (margin / grounding.uncertainty_margin).max(0.0);
    let boost = (grounding.bias_scale * grounding.confidence * urgency).min(grounding.bias_cap);
    for candidate in candidates.iter_mut() {
        if grounding.preferred_tokens.contains(&candidate.id) {
            candidate.logit += boost;
        }
    }
    array.selected = -1;
    array.sorted = false;
    if top_id(candidates) != Some(before) {
        context.stats.deflections += 1;
    }
}

fn top_two(candidates: &[llama_token_data]) -> Option<(llama_token, f32, f32)> {
    let mut best = None;
    let mut top = f32::NEG_INFINITY;
    let mut second = f32::NEG_INFINITY;
    for candidate in candidates {
        if !candidate.logit.is_finite() {
            continue;
        }
        if candidate.logit > top {
            second = top;
            top = candidate.logit;
            best = Some(candidate.id);
        } else if candidate.logit > second {
            second = candidate.logit;
        }
    }
    if second.is_finite() {
        best.map(|token| (token, top, second))
    } else {
        None
    }
}

fn top_id(candidates: &[llama_token_data]) -> Option<llama_token> {
    candidates
        .iter()
        .filter(|candidate| candidate.logit.is_finite())
        .max_by(|left, right| left.logit.total_cmp(&right.logit))
        .map(|candidate| candidate.id)
}

/// Turn grounding terms into preferred token ids, using the reference's rule:
/// split on anything non-alphanumeric, drop words shorter than four characters,
/// lowercase, and keep only words that encode to exactly one token. Sorted,
/// deduped, capped.
pub fn preferred_tokens<F>(terms: &[String], mut single_token: F) -> Vec<llama_token>
where
    F: FnMut(&str) -> Option<llama_token>,
{
    let mut tokens = Vec::new();
    for term in terms {
        for word in term.split(|character: char| !character.is_alphanumeric()) {
            if word.chars().count() < 4 {
                continue;
            }
            if let Some(token) = single_token(&word.to_lowercase()) {
                tokens.push(token);
            }
        }
    }
    tokens.sort_unstable();
    tokens.dedup();
    tokens.truncate(64);
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(logits: &[f32]) -> Vec<llama_token_data> {
        logits
            .iter()
            .enumerate()
            .map(|(id, logit)| llama_token_data {
                id: id as llama_token,
                logit: *logit,
                p: 0.0,
            })
            .collect()
    }

    fn array_of(data: &mut [llama_token_data]) -> llama_token_data_array {
        llama_token_data_array {
            data: data.as_mut_ptr(),
            size: data.len(),
            selected: -1,
            sorted: false,
        }
    }

    fn grounding(tokens: Vec<llama_token>) -> TokenGrounding {
        TokenGrounding {
            preferred_tokens: tokens,
            receipt_hash: "a".repeat(64),
            confidence: 1.0,
            uncertainty_margin: 1.5,
            bias_scale: 0.45,
            bias_cap: 0.75,
        }
    }

    #[test]
    fn passthrough_leaves_logits_byte_identical() {
        let mut context = HookContext {
            grounding: None,
            stats: HookStats::default(),
        };
        let original = [0.0f32, 1.0, 0.4, -1.0];
        let mut data = candidates(&original);
        let mut array = array_of(&mut data);
        apply_direct(&mut context, &mut data, &mut array);
        for (candidate, before) in data.iter().zip(original) {
            assert_eq!(candidate.logit.to_bits(), before.to_bits());
        }
        assert_eq!(context.stats.raw_logit_passes, 1);
        assert_eq!(context.stats.fires, 0);
        assert_eq!(context.stats.deflections, 0);
    }

    #[test]
    fn a_decided_step_is_left_alone() {
        let mut context = HookContext {
            grounding: Some(grounding(vec![2])),
            stats: HookStats::default(),
        };
        // Top-2 margin 6.0 is far above the 1.5 gate.
        let mut data = candidates(&[0.0, 6.0, 1.0, -1.0]);
        let mut array = array_of(&mut data);
        apply_direct(&mut context, &mut data, &mut array);
        assert_eq!(context.stats.fires, 0);
        assert_eq!(data[2].logit, 1.0);
    }

    #[test]
    fn an_undecided_step_boosts_preferred_tokens_within_the_cap() {
        let mut context = HookContext {
            grounding: Some(grounding(vec![2])),
            stats: HookStats::default(),
        };
        // Margin is 10.0 - 9.75 = 0.25, well inside the 1.5 gate, so the filter
        // fires. Urgency = 1 - 0.25/1.5 = 0.8333..., so the boost is
        // 0.45 * 0.8333... = 0.375, which lifts token 2 above token 1.
        let mut data = candidates(&[0.0, 10.0, 9.75, -1.0]);
        let mut array = array_of(&mut data);
        apply_direct(&mut context, &mut data, &mut array);
        assert_eq!(context.stats.fires, 1);
        assert_eq!(context.stats.deflections, 1);
        let boost = data[2].logit - 9.75;
        assert!((boost - 0.375).abs() < 0.001, "boost was {boost}");
        assert!(data[2].logit > data[1].logit, "the argmax should have moved");
        // The un-preferred token is untouched.
        assert_eq!(data[1].logit, 10.0);
        assert_eq!(array.selected, -1, "selection is left to the sampler stage");
        assert!(!array.sorted);
    }

    #[test]
    fn the_boost_never_exceeds_the_adapter_cap() {
        // A scale larger than the cap makes urgency irrelevant: the cap binds.
        let mut context = HookContext {
            grounding: Some(TokenGrounding {
                bias_scale: 2.0,
                bias_cap: 0.75,
                ..grounding(vec![1])
            }),
            stats: HookStats::default(),
        };
        let mut data = candidates(&[5.0, 4.9999995, 0.0]);
        let mut array = array_of(&mut data);
        apply_direct(&mut context, &mut data, &mut array);
        let boost = data[1].logit - 4.9999995;
        assert!((boost - 0.75).abs() < 0.001, "boost {boost} was not capped");
    }

    #[test]
    fn preferred_tokens_follow_the_reference_rule() {
        let terms = vec![
            "Candidate generation".to_string(),
            "candidate generation".to_string(),
            "a".to_string(),
            "the".to_string(),
        ];
        // A fake tokenizer: one token per known lowercase word, nothing else.
        let tokens = preferred_tokens(&terms, |word| match word {
            "candidate" => Some(11),
            "generation" => Some(12),
            _ => None,
        });
        assert_eq!(tokens, vec![11, 12], "deduped, sorted, short words dropped");
    }

    #[test]
    fn preferred_tokens_are_capped() {
        let terms: Vec<String> = (0..200).map(|index| format!("word{index:04}")).collect();
        let tokens = preferred_tokens(&terms, |_| Some(7));
        assert_eq!(tokens, vec![7]);
    }

    #[test]
    fn a_binding_without_a_receipt_hash_is_refused() {
        let mut binding = grounding(vec![1]);
        binding.receipt_hash = "short".into();
        assert!(LightconeFilter::with_lightcone(binding).is_err());
    }

    #[test]
    fn a_binding_without_tokens_is_refused() {
        assert!(LightconeFilter::with_lightcone(grounding(vec![])).is_err());
    }
}
