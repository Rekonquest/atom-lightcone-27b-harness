//! Local Prism model execution with a typed post-weights Atom experiment and
//! the direct post-weights Lightcone binding.

// The diagnostic payload is wide by design — every quantity reported
// separately, anti-collapse — so the json! macro needs room to expand it.
#![recursion_limit = "512"]

mod consumers;
mod creative;
mod reasoning_brief;
mod ffi;
mod ledger;
mod post_weights;

use atom_executive::Outcome;
use atom_orchestrator::{
    CreativeInput, Engine, Input, LawProblem, LawRegistry, Release, Report, Sign, SourcedLawReport,
    Status, check_release, reinforces, released_text,
};
use ffi::*;
use ledger::{WarrantEntry, WarrantLedger};
use post_weights::{HookStats, LightconeFilter, TokenGrounding};
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::path::Path;

struct Model {
    model: *mut llama_model,
    context: *mut llama_context,
    vocab: *const llama_vocab,
}

unsafe extern "C" fn diagnostic_log(level: i32, message: *const c_char, _: *mut c_void) {
    if level >= 3 && level != 5 && !message.is_null() {
        eprint!("{}", unsafe { CStr::from_ptr(message) }.to_string_lossy());
    }
}

impl Model {
    fn load(path: &Path) -> Result<Self, String> {
        let name = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| "model path contains NUL")?;
        unsafe {
            llama_log_set(Some(diagnostic_log), std::ptr::null_mut());
            ggml_log_set(Some(diagnostic_log), std::ptr::null_mut());
            llama_backend_init();
            let mut params = llama_model_default_params();
            params.n_gpu_layers = 99;
            let model = llama_model_load_from_file(name.as_ptr(), params);
            if model.is_null() {
                return Err("Prism could not load the model".into());
            }
            let mut settings = llama_context_default_params();
            settings.n_ctx = 4096;
            settings.n_batch = 512;
            settings.n_ubatch = 512;
            settings.n_threads = 16;
            settings.n_threads_batch = 16;
            let context = llama_init_from_model(model, settings);
            if context.is_null() {
                llama_model_free(model);
                return Err("Prism could not create a context".into());
            }
            let vocab = llama_model_get_vocab(model);
            if vocab.is_null() {
                llama_free(context);
                llama_model_free(model);
                return Err("Prism model has no vocabulary".into());
            }
            Ok(Self {
                model,
                context,
                vocab,
            })
        }
    }

    fn tokenize(&self, text: &str) -> Result<Vec<llama_token>, String> {
        let length = i32::try_from(text.len()).map_err(|_| "prompt too long")?;
        let mut tokens = vec![0; text.len().saturating_add(64)];
        let mut capacity = i32::try_from(tokens.len()).map_err(|_| "prompt too long")?;
        let mut count = unsafe {
            llama_tokenize(
                self.vocab,
                text.as_ptr().cast::<c_char>(),
                length,
                tokens.as_mut_ptr(),
                capacity,
                false,
                true,
            )
        };
        if count < 0 {
            tokens.resize((-count) as usize, 0);
            capacity = i32::try_from(tokens.len()).map_err(|_| "prompt too long")?;
            count = unsafe {
                llama_tokenize(
                    self.vocab,
                    text.as_ptr().cast::<c_char>(),
                    length,
                    tokens.as_mut_ptr(),
                    capacity,
                    false,
                    true,
                )
            };
        }
        if count <= 0 {
            return Err("Prism tokenization failed".into());
        }
        tokens.truncate(count as usize);
        Ok(tokens)
    }

    fn decode(&mut self, tokens: &[llama_token]) -> Result<(), String> {
        let mut owned = tokens.to_vec();
        let count = i32::try_from(owned.len()).map_err(|_| "decode batch too large")?;
        let code =
            unsafe { llama_decode(self.context, llama_batch_get_one(owned.as_mut_ptr(), count)) };
        if code == 0 {
            Ok(())
        } else {
            Err(format!("Prism decode failed: {code}"))
        }
    }

    fn piece(&self, token: llama_token) -> Result<String, String> {
        let mut buffer = vec![0 as c_char; 256];
        let mut count = unsafe {
            llama_token_to_piece(
                self.vocab,
                token,
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                0,
                true,
            )
        };
        if count < 0 {
            buffer.resize((-count) as usize, 0);
            count = unsafe {
                llama_token_to_piece(
                    self.vocab,
                    token,
                    buffer.as_mut_ptr(),
                    buffer.len() as i32,
                    0,
                    true,
                )
            };
        }
        if count < 0 {
            return Err("Prism token conversion failed".into());
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), count as usize) };
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn reset(&mut self) -> Result<(), String> {
        let memory = unsafe { llama_get_memory(self.context) };
        if memory.is_null() || !unsafe { llama_memory_seq_rm(memory, 0, -1, -1) } {
            return Err("Prism context reset failed".into());
        }
        Ok(())
    }

    fn logits(&self) -> Result<&[f32], String> {
        let length = unsafe { llama_vocab_n_tokens(self.vocab) };
        let data = unsafe { llama_get_logits_ith(self.context, -1) };
        if length <= 0 || data.is_null() {
            return Err("Prism did not expose post-weights logits".into());
        }
        Ok(unsafe { std::slice::from_raw_parts(data, length as usize) })
    }

    /// The model's own token for a lowercase word, only when that word encodes
    /// to exactly one token — the reference rule for preferred tokens.
    fn single_token(&self, word: &str) -> Option<llama_token> {
        let tokens = self.tokenize(&format!(" {word}")).ok()?;
        (tokens.len() == 1).then(|| tokens[0])
    }

    fn run(&mut self, prompt: &str, query: &str, assistant_suffix: &str, budget: usize, config: ArmConfig) -> Result<Arm, String> {
        self.reset()?;
        let tokens = self.tokenize(prompt)?;
        if tokens.len() + budget >= 4096 {
            return Err("prompt and generation exceed context".into());
        }
        for chunk in tokens.chunks(512) {
            self.decode(chunk)?;
        }
        let mut selector = Selector::new(
            0x8d49_42d5,
            config.cognitive_input,
            config.sourced_laws,
            config.creative,
            config.experimental_temperature,
        );
        selector.prepare(self.logits()?)?;
        // The direct post-weights binding: a registered Lightcone sampler
        // applied over raw logits, ahead of this harness's own selection stage.
        // With no grounding (unknown or out-of-cone query) it is exact
        // pass-through and cannot change a single logit.
        let binding_attached = config.binding.is_some();
        let mut filter = match config.binding {
            Some(grounding) => LightconeFilter::with_lightcone(grounding)?,
            None => LightconeFilter::passthrough()?,
        };
        let (context, context_kind) = if config.reasoning_brief {
            // The executive's answer, not the goal's lane, is the material —
            // and ahead of it, the cone's own admitted claims.
            match selector
                .outcome
                .as_ref()
                .and_then(|outcome| reasoning_brief::with_geometry(config.admitted_geometry.as_deref(), outcome))
            {
                Some(rendered) => (Some(rendered), "reasoning-brief"),
                None => (None, "none"),
            }
        } else if config.experimental_structured_translation {
            match selector.law_report.as_ref().and_then(render_law_context) {
                Some(rendered) => (Some(rendered), "structured-translation"),
                None => (None, "none"),
            }
        } else {
            (None, "none")
        };
        if let Some(context) = &context {
            self.reset()?;
            let grounded_prompt = format!(
                "<|im_start|>system\n{context}<|im_end|>\n\
                <|im_start|>user\n{query}<|im_end|>\n<|im_start|>assistant\n{assistant_suffix}"
            );
            let grounded_tokens = self.tokenize(&grounded_prompt)?;
            if grounded_tokens.len() + budget >= 4096 {
                return Err("grounded prompt and generation exceed context".into());
            }
            for chunk in grounded_tokens.chunks(512) {
                self.decode(chunk)?;
            }
        }
        let mut output = String::new();
        let mut selected = 0;
        // Reused per-step: the full-vocabulary candidate array the sampler sees,
        // and the logits slice this harness's selection stage reads afterwards.
        let mut candidates: Vec<llama_token_data> = Vec::new();
        let mut filtered: Vec<f32> = Vec::new();
        for _ in 0..budget {
            {
                let logits = self.logits()?;
                candidates.clear();
                candidates.extend(logits.iter().enumerate().map(|(id, logit)| {
                    llama_token_data {
                        id: id as llama_token,
                        logit: *logit,
                        p: 0.0,
                    }
                }));
                let mut array = llama_token_data_array {
                    data: candidates.as_mut_ptr(),
                    size: candidates.len(),
                    selected: -1,
                    sorted: false,
                };
                filter.apply(&mut array)?;
                filtered.clear();
                filtered.extend(candidates.iter().map(|candidate| candidate.logit));
            }
            let token = selector.select(&filtered)?;
            if unsafe { llama_vocab_is_eog(self.vocab, token) } {
                break;
            }
            output.push_str(&self.piece(token)?);
            self.decode(&[token])?;
            selected += 1;
        }
        Ok(Arm {
            text: output,
            selected,
            postweights_passes: selector.passes,
            counterfactual_deflections: selector.deflections,
            outcome: selector.outcome,
            sourced_law_report: selector.law_report,
            context_kind,
            binding_attached,
            binding_stats: filter.stats(),
        })
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            llama_free(self.context);
            llama_model_free(self.model);
        }
    }
}

/// One arm's post-weights options: what Atom dispatches, and what — if
/// anything — the passive Lightcone binds at the sampling boundary.
struct ArmConfig {
    cognitive_input: Option<Input>,
    sourced_laws: Option<(LawRegistry, LawProblem)>,
    /// The creative dispatch, when the operator supplied a brief. It re-runs
    /// the reasoning lanes so the report it consumes is the one it was given;
    /// the cost is CPU-only and the isolation is worth it.
    creative: Option<CreativeInput>,
    binding: Option<TokenGrounding>,
    experimental_temperature: bool,
    experimental_structured_translation: bool,
    /// Render the executive's answer into the model's context, so the model
    /// reasons from the lanes instead of guessing. Off by default: a diagnostic
    /// revision is compared against its own baseline.
    reasoning_brief: bool,
    /// The admitted geometry rendered once for the brief: the cone's typed
    /// claims with labels and provenance. Only read when `reasoning_brief` is
    /// set, so the baseline stays byte-identical to the pre-brief runner.
    admitted_geometry: Option<String>,
}

#[derive(Debug)]
struct Arm {
    text: String,
    selected: usize,
    postweights_passes: usize,
    counterfactual_deflections: usize,
    /// The executive's answer: which rung answered, every lane's own position,
    /// the bridges, and the creative paths when reasoning could not stand.
    outcome: Option<Outcome>,
    sourced_law_report: Option<SourcedLawReport>,
    /// Which mechanism put material in the model's context, if any.
    context_kind: &'static str,
    binding_attached: bool,
    binding_stats: HookStats,
}

struct Selector {
    state: u64,
    signal: usize,
    cognitive_input: Option<Input>,
    sourced_laws: Option<(LawRegistry, LawProblem)>,
    /// The operator's creative material, handed to the executive. The executive
    /// decides whether the creative lane runs; the host does not.
    brief: Option<CreativeInput>,
    outcome: Option<Outcome>,
    law_report: Option<SourcedLawReport>,
    experimental_temperature: bool,
    passes: usize,
    deflections: usize,
}

impl Selector {
    fn new(
        seed: u64,
        cognitive_input: Option<Input>,
        sourced_laws: Option<(LawRegistry, LawProblem)>,
        brief: Option<CreativeInput>,
        experimental_temperature: bool,
    ) -> Self {
        Self {
            state: seed,
            signal: 0,
            cognitive_input,
            sourced_laws,
            brief,
            outcome: None,
            law_report: None,
            experimental_temperature,
            passes: 0,
            deflections: 0,
        }
    }

    fn draw(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state as f64) / (u64::MAX as f64)
    }

    fn prepare(&mut self, raw: &[f32]) -> Result<(), String> {
        if raw.is_empty() {
            return Err("post-weights logits absent".into());
        }
        if let Some((registry, problem)) = self.sourced_laws.take() {
            let report = atom_orchestrator::run_laws(&Engine, &registry, &problem)
                .map_err(|error| format!("post-weights sourced-law dispatch failed: {error:?}"))?;
            self.law_report = Some(report);
        }
        if let Some(input) = self.cognitive_input.take() {
            let outcome = atom_executive::run(&Engine, input, self.brief.take())
                .map_err(|error| format!("post-weights executive dispatch failed: {error:?}"))?;
            self.signal = outcome.lanes.iter().map(|lane| lane.paths.len()).sum();
            self.outcome = Some(outcome);
        }
        Ok(())
    }

    fn select(&mut self, raw: &[f32]) -> Result<llama_token, String> {
        self.passes += 1;
        self.prepare(raw)?;
        let mut ranked: Vec<(i32, f32)> = raw
            .iter()
            .enumerate()
            .filter(|(_, logit)| logit.is_finite())
            .map(|(id, logit)| (id as i32, *logit))
            .collect();
        if ranked.is_empty() {
            return Err("all post-weights logits invalid".into());
        }
        ranked.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        ranked.truncate(20);
        let draw = self.draw();
        let bare = sample(&ranked, 1.0, draw)?;
        let temperature =
            if self.experimental_temperature && self.outcome.is_some() && self.signal > 0 {
                1.0 / (1.0 + 0.15 * self.signal.min(3) as f64)
            } else {
                1.0
            };
        let chosen = sample(&ranked, temperature, draw)?;
        if chosen != bare {
            self.deflections += 1;
        }
        Ok(chosen)
    }
}

fn sample(ranked: &[(i32, f32)], temperature: f64, draw: f64) -> Result<i32, String> {
    let top = ranked.first().ok_or("no token candidates")?.1 as f64;
    let weights: Vec<f64> = ranked
        .iter()
        .map(|(_, value)| (((*value as f64) - top) / temperature).exp())
        .collect();
    let full_total: f64 = weights.iter().sum();
    if !full_total.is_finite() || full_total <= 0.0 {
        return Err("invalid token distribution".into());
    }
    let mut cumulative = 0.0;
    let admitted = weights
        .iter()
        .take_while(|weight| {
            let keep = cumulative < 0.95;
            cumulative += **weight / full_total;
            keep
        })
        .count();
    let total: f64 = weights.iter().take(admitted).sum();
    let mut remaining = draw * total;
    for ((id, _), weight) in ranked.iter().zip(weights).take(admitted) {
        remaining -= weight;
        if remaining <= 0.0 {
            return Ok(*id);
        }
    }
    ranked
        .get(admitted.saturating_sub(1))
        .map(|(id, _)| *id)
        .ok_or_else(|| "no token candidates".into())
}

/// Which lanes disagreed, named. A conflict is reported, never averaged.
fn conflicting_lanes(outcome: Option<&Outcome>) -> Vec<String> {
    let Some(outcome) = outcome else { return Vec::new() };
    if !outcome.conflict {
        return Vec::new();
    }
    outcome
        .positions
        .iter()
        .filter(|position| position.status == Status::SupportedPath && position.direction.is_some())
        .map(|position| format!("{:?}:{:?}", position.lane, position.direction))
        .collect()
}

fn render_law_context(report: &SourcedLawReport) -> Option<String> {
    if report.status != Status::SupportedPath {
        return None;
    }
    let direction = report.direction?;
    let path = report
        .considered_paths
        .iter()
        .filter(|path| {
            (path.direction == 1 && direction == Sign::Supports)
                || (path.direction == -1 && direction == Sign::Opposes)
        })
        .max_by(|left, right| left.strength.total_cmp(&right.strength))?;
    Some(format!(
        "Sourced causal-model artifact (not observed fact).\n\
         Law IDs: {:?}. Direction: {:?}.\n\
         Feature path: {:?}. Context keys by law: {:?}.\n\
         Evidence count: {}. Confidence: {:.12}. Persistence: {:.12}.\n",
        path.law_ids,
        direction,
        path.feature_path,
        path.condition_path,
        path.evidence_ids.len(),
        path.confidence,
        path.persistence,
    ))
}

fn required(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {name}"))
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let model_path = required(&mut args, "model GGUF")?;
    let pack = required(&mut args, "Lightcone pack")?;
    let query = required(&mut args, "query")?;
    let cause = required(&mut args, "cause node or -")?;
    let effect = required(&mut args, "effect node")?;
    let goal = harness_cli::goal(&required(&mut args, "goal")?)?;
    let budget = args
        .next()
        .map(|v| v.parse::<usize>())
        .transpose()
        .map_err(|_| "invalid token budget")?
        .unwrap_or(128);
    if budget == 0 || budget > 256 {
        return Err("token budget must be within 1..=256".into());
    }
    let mut next = args.next();
    let json_output = next.as_deref() == Some("--json");
    if json_output {
        next = args.next();
    }
    let no_think = next.as_deref() == Some("--no-think");
    if no_think {
        next = args.next();
    }
    let experimental_temperature = next.as_deref() == Some("--experimental-temperature");
    if experimental_temperature {
        next = args.next();
    }
    let experimental_structured_translation =
        next.as_deref() == Some("--experimental-structured-translation");
    if experimental_structured_translation {
        next = args.next();
    }
    // Diagnostic parity with the revision that had no sampling-boundary binding.
    let lightcone_tokens = next.as_deref() != Some("--no-lightcone-tokens");
    if !lightcone_tokens {
        next = args.next();
    }
    let mut ledger_path = ledger::default_ledger_path();
    if next.as_deref() == Some("--ledger") {
        let value = required(&mut args, "warrant ledger path")?;
        ledger_path = std::path::PathBuf::from(value);
        next = args.next();
    }
    let mut creative_path: Option<std::path::PathBuf> = None;
    if next.as_deref() == Some("--creative") {
        let value = required(&mut args, "creative brief path")?;
        creative_path = Some(std::path::PathBuf::from(value));
        next = args.next();
    }
    // The two lanes that need more than a goal: transfer needs an explicit
    // analogy mapping, alter-premise needs the link to remove. Without these
    // the engine refuses the question — correct, but unreachable.
    let reasoning_brief_flag = next.as_deref() == Some("--reasoning-brief");
    if reasoning_brief_flag {
        next = args.next();
    }
    let mut removed_link: Option<String> = None;
    if next.as_deref() == Some("--removed-link") {
        removed_link = Some(required(&mut args, "removed link id")?);
        next = args.next();
    }
    let mut analogy: Option<atom_orchestrator::Analogy> = None;
    if next.as_deref() == Some("--analogy") {
        let source_cause = required(&mut args, "analogy source cause")?;
        let source_effect = required(&mut args, "analogy source effect")?;
        let target_cause = (cause != "-")
            .then(|| cause.clone())
            .ok_or("--analogy needs a question cause, because the mapping targets it")?;
        analogy = Some(atom_orchestrator::Analogy {
            source_cause,
            source_effect,
            target_cause,
            target_effect: effect.clone(),
        });
        next = args.next();
    }
    let sourced_laws = match next {
        None => None,
        Some(flag) if flag == "--law" => {
            let law_cause = required(&mut args, "sourced-law cause feature")?;
            let law_effect = required(&mut args, "sourced-law effect feature")?;
            let law_domain = required(&mut args, "sourced-law domain or -")?;
            let registry = harness_cli::causal_laws::load_copied_worlds(
                &harness_cli::causal_laws::copied_world_root(),
            )?;
            Some((
                registry,
                LawProblem {
                    cause: law_cause,
                    effect: law_effect,
                    domain: (law_domain != "-").then_some(law_domain),
                    conditions: vec![],
                    allowed_domains: vec![],
                },
            ))
        }
        Some(_) => return Err("expected --law or end of arguments".into()),
    };
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    if experimental_structured_translation && sourced_laws.is_none() {
        return Err("structured translation requires --law".into());
    }
    let turn = harness_cli::admit(
        &pack,
        &query,
        (cause != "-").then_some(cause),
        effect,
        goal,
        removed_link,
        analogy,
    )?;
    let receipt = turn.input.admission.receipt_id.clone();
    let manifest = harness_cli::text(&turn.document["receipt"], "manifest_hash")?;
    let terms = harness_cli::grounding_terms(&turn.document)?;
    let boundary_input = turn.input.clone();
    // The reference lane's evidence: the admitted typed claims, labels included,
    // rendered once here where the document is in scope. Declarations with
    // provenance only — no node prose, no instructions.
    let mut labels = std::collections::BTreeMap::new();
    for node in harness_cli::array(&turn.document, "nodes")? {
        let id = node.get("node_id").and_then(|value| value.as_str()).unwrap_or_default();
        if let Some(label) = node.get("label").and_then(|value| value.as_str()) {
            labels.insert(id.to_string(), label.to_string());
        }
    }
    let admitted_geometry =
        reasoning_brief::render_geometry(&boundary_input.admission.links, &labels);
    let creative_input = match &creative_path {
        Some(path) => Some(creative::load(path, boundary_input.clone())?),
        None => None,
    };
    let cognitive_input = turn.input;
    let assistant_suffix = if no_think {
        "<think>\n\n</think>\n\n"
    } else {
        ""
    };
    let prompt =
        format!("<|im_start|>user\n{query}<|im_end|>\n<|im_start|>assistant\n{assistant_suffix}");
    let mut native = Model::load(Path::new(&model_path))?;
    // The admitted vocabulary becomes a bounded, adapter-owned preference set.
    // An admission that admitted nothing — the unknown and out-of-cone controls
    // — leaves this empty, and empty grounding is exact pass-through.
    let preferred = if lightcone_tokens {
        post_weights::preferred_tokens(&terms, |word| native.single_token(word))
    } else {
        Vec::new()
    };
    let preferred_count = preferred.len();
    let binding = (!preferred.is_empty()).then(|| TokenGrounding {
        preferred_tokens: preferred,
        receipt_hash: receipt.clone(),
        confidence: 1.0,
        uncertainty_margin: 1.5,
        bias_scale: 0.45,
        bias_cap: 0.75,
    });
    let bare = native.run(
        &prompt,
        &query,
        assistant_suffix,
        budget,
        ArmConfig {
            cognitive_input: None,
            sourced_laws: None,
            creative: None,
            binding: None,
            experimental_temperature: false,
            experimental_structured_translation: false,
            reasoning_brief: false,
            admitted_geometry: None,
        },
    )?;
    let atom = native.run(
        &prompt,
        &query,
        assistant_suffix,
        budget,
        ArmConfig {
            cognitive_input: Some(cognitive_input),
            sourced_laws,
            creative: creative_input,
            binding,
            experimental_temperature,
            experimental_structured_translation,
            reasoning_brief: reasoning_brief_flag,
            admitted_geometry: if reasoning_brief_flag { admitted_geometry } else { None },
        },
    )?;
    // The output-side boundary and the feedback lane, both engine-side. Both
    // arms are classified against the same admitted claim so the comparison
    // stays isolated; only the grounded arm may reinforce.
    let engine_absent = Report {
        receipt_id: receipt.clone(),
        question_id: String::new(),
        lanes: Vec::new(),
        factual_claim_authority: false,
    };
    let bare_check = check_release(&boundary_input, &engine_absent, &bare.text);
    // The output lane reads the same lanes the executive ran, so the check and
    // the answer cannot disagree about what was admitted.
    let atom_report = atom.outcome.as_ref().map(|outcome| Report {
        receipt_id: receipt.clone(),
        question_id: String::new(),
        lanes: outcome.lanes.clone(),
        factual_claim_authority: false,
    });
    let atom_check = check_release(
        &boundary_input,
        atom_report.as_ref().unwrap_or(&engine_absent),
        &atom.text,
    );
    let mut ledger = WarrantLedger::load(&ledger_path)?;
    let mut reinforced = false;
    let mut ledger_note = String::new();
    if reinforces(&atom_check)
        && let Some(claim_id) = &atom_check.claim
    {
        let world = boundary_input
            .admission
            .links
            .iter()
            .find(|link| &link.id == claim_id)
            .map(|link| link.world.clone())
            .unwrap_or_default();
        let entry = WarrantEntry {
            claim_id: claim_id.clone(),
            world,
            polarity: atom_check
                .claim_polarity
                .map(|sign| format!("{sign:?}"))
                .unwrap_or_default(),
            claim_class: atom_check
                .claim_class
                .map(|class| format!("{class:?}"))
                .unwrap_or_default(),
            reinforcements: 0,
            last_receipt: receipt.clone(),
        };
        match ledger.reinforce(entry) {
            Ok(_) => {
                reinforced = true;
                ledger.save(&ledger_path)?;
            }
            Err(error) => ledger_note = error,
        }
    }
    let released = released_text(&atom_check, &atom.text);
    // Consumers for what the engine already computes: per-lane evidence
    // weights, the prediction relation, contested-evidence status, and the
    // creative frontier. Each is reported on its own; none is blended.
    let lane_weights: Vec<serde_json::Value> = atom
        .outcome
        .as_ref()
        .map(|outcome| {
            consumers::lane_summaries(&outcome.lanes)
                .into_iter()
                .map(|summary| {
                    serde_json::json!({
                        "lane": format!("{:?}", summary.lane),
                        "status": format!("{:?}", summary.status),
                        "direction": summary.direction.map(|sign| format!("{sign:?}")),
                        "evidence_weight": summary.evidence_weight,
                        "paths": summary.paths,
                        "hypotheses": summary.hypotheses,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let prediction_list: Vec<serde_json::Value> = atom
        .outcome
        .as_ref()
        .map(|outcome| {
            consumers::prediction_paths(&boundary_input.admission, &outcome.lanes)
                .into_iter()
                .map(|path| {
                    serde_json::json!({
                        "link_ids": path.link_ids,
                        "source_ids": path.source_ids,
                        "sign": format!("{:?}", path.sign),
                        "established": path.established,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let frontier: Vec<serde_json::Value> = atom
        .outcome
        .as_ref()
        .map(|outcome| {
            outcome
                .creative
                .iter()
                .take(8)
                .map(|path| {
                    serde_json::json!({
                        "label": path.label,
                        "candidate_id": path.candidate_id,
                        "status": path.status,
                        "probability_kind": path.probability_kind,
                        "probability_score": path.probability_score,
                        "uncertainty_interval": [path.uncertainty_interval.0, path.uncertainty_interval.1],
                        "expected_creative_utility": path.expected_creative_utility,
                        "required_coverage": path.required_coverage,
                        "novelty": path.novelty,
                        "value": path.value,
                        "hard_valid": path.hard_valid,
                        "concept_ids": path.concept_ids.clone(),
                        "spaces": path.spaces.clone(),
                        "source_ids": path.source_ids.clone(),
                        "factual_claim_authority": path.factual_claim_authority,
                        "proposal_only": path.proposal_only,
                        "choice_records_intent_only": path.choice_records_intent_only,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let contested = atom
        .outcome
        .as_ref()
        .is_some_and(|outcome| consumers::contested_evidence(&outcome.lanes));
    if json_output {
        let law = atom.sourced_law_report.as_ref();
        let payload = serde_json::json!({
            "schema": 1,
            "model_path": model_path,
            "pack_manifest_hash": manifest,
            "passive_receipt_hash": receipt,
            "no_think": no_think,
            "experimental_temperature_enabled": experimental_temperature,
            "experimental_structured_translation_enabled": experimental_structured_translation,
            "lightcone_token_binding": atom.binding_attached,
            "lightcone_preferred_tokens": preferred_count,
            "lightcone_raw_logit_passes": atom.binding_stats.raw_logit_passes,
            "lightcone_fires": atom.binding_stats.fires,
            "lightcone_deflections": atom.binding_stats.deflections,
            "lightcone_passive": true,
            "lightcone_holds_verdict": false,
            "boundary": "weights/raw_logits -> passive_lightcone -> token_selection",
            "mcp_used": false,
            "admitted_links": boundary_input.admission.links.len(),
            "out_lane_verdict": format!("{:?}", atom_check.verdict),
            "out_lane_release": format!("{:?}", atom_check.release),
            "out_lane_reason": atom_check.reason,
            "out_lane_claim": atom_check.claim.clone(),
            "out_lane_claim_polarity": atom_check.claim_polarity.map(|sign| format!("{sign:?}")),
            "out_lane_claim_class": atom_check.claim_class.map(|class| format!("{class:?}")),
            "out_lane_engine_status": atom_check.engine_status.map(|status| format!("{status:?}")),
            "out_lane_engine_direction": atom_check.engine_direction.map(|sign| format!("{sign:?}")),
            "out_lane_asserted": atom_check.asserted.clone(),
            "out_lane_negated": atom_check.negated.clone(),
            "out_lane_factual_claim_authority": atom_check.factual_claim_authority,
            "bare_out_lane_verdict": format!("{:?}", bare_check.verdict),
            "bare_out_lane_asserted": bare_check.asserted.clone(),
            "bare_out_lane_negated": bare_check.negated.clone(),
            "bare_out_lane_release": format!("{:?}", bare_check.release),
            "released": atom_check.release == Release::Released,
            "reinforced": reinforced,
            "warrant_ledger_claim": atom_check.claim.clone(),
            "warrant_ledger_reinforcements": atom_check
                .claim
                .as_ref()
                .map_or(0, |claim| ledger.count(claim)),
            "warrant_ledger_claims": ledger.claims(),
            "warrant_ledger_total": ledger.total_reinforcements(),
            "warrant_ledger_path": ledger_path.display().to_string(),
            "warrant_ledger_note": ledger_note.clone(),
            "released_text": released,
            "lane_evidence_weights": lane_weights,
            "contested_evidence": contested,
            "prediction_paths": prediction_list,
            "executive_dispatched": atom.outcome.is_some(),
            "executive_rung": atom.outcome.as_ref().map(|outcome| format!("{:?}", outcome.rung)),
            "executive_status": atom.outcome.as_ref().map(|outcome| format!("{:?}", outcome.status)),
            "executive_direction": atom
                .outcome
                .as_ref()
                .and_then(|outcome| outcome.direction)
                .map(|direction| format!("{direction:?}")),
            "executive_conflict": atom.outcome.as_ref().is_some_and(|outcome| outcome.conflict),
            "executive_winner_declared": atom.outcome.as_ref().is_some_and(|outcome| outcome.winner_declared),
            "executive_selected_lanes": atom
                .outcome
                .as_ref()
                .map(|outcome| outcome.selection.lanes.iter().map(|lane| format!("{lane:?}")).collect::<Vec<_>>())
                .unwrap_or_default(),
            "executive_conflicting_lanes": conflicting_lanes(atom.outcome.as_ref()),
            "executive_bridges": atom
                .outcome
                .as_ref()
                .map(|outcome| outcome.bridges.iter().map(|bridge| serde_json::json!({
                    "from": format!("{:?}", bridge.from),
                    "to": format!("{:?}", bridge.to),
                    "relation": format!("{:?}", bridge.relation),
                    "link_ids": bridge.link_ids.clone(),
                    "source_ids": bridge.source_ids.clone(),
                    "evidence_class": format!("{:?}", bridge.evidence_class),
                })).collect::<Vec<_>>())
                .unwrap_or_default(),
            "creative_paths": frontier,
            "creative_disclaimer": atom_executive::CreativePath::DISCLAIMER,
            "creative_path_count": atom.outcome.as_ref().map_or(0, |outcome| outcome.creative.len()),
            "creative_factual_claim_authority": false,
            "executive_factual_claim_authority": false,
            "lightcone_sourced_law_typed_join": false,
            "postweights_executive_dispatch": atom.outcome.is_some(),
            "postweights_sourced_law_dispatch": law.is_some(),
            "sourced_law_strategy": law.map(|report| report.plan.strategy),
            "sourced_law_status": law.map(|report| format!("{:?}", report.status)),
            "sourced_law_direction": law.and_then(|report| report.direction)
                .map(|sign| format!("{sign:?}")),
            "sourced_law_confidence": law.and_then(|report| report.confidence),
            "sourced_law_paths": law.map(|report| report.considered_paths.len()),
            "sourced_law_ids": law.map(|report| report.considered_paths.iter()
                .flat_map(|path| path.law_ids.iter().cloned()).collect::<Vec<_>>()),
            "sourced_law_context_keys": law.map(|report| report.considered_paths.iter()
                .flat_map(|path| path.condition_path.iter().flatten().cloned())
                .collect::<Vec<_>>()),
            "structured_translation_used": atom.context_kind == "structured-translation",
            "reasoning_brief_used": atom.context_kind == "reasoning-brief",
            "bare": {"text": bare.text, "tokens": bare.selected,
                "postweights_passes": bare.postweights_passes},
            "atom": {"text": atom.text, "tokens": atom.selected,
                "postweights_passes": atom.postweights_passes,
                "deflections": atom.counterfactual_deflections},
            "factual_claim_authority": false
        });
        println!(
            "{}",
            serde_json::to_string(&payload)
                .map_err(|error| format!("JSON output failed: {error}"))?
        );
        return Ok(());
    }
    println!("pack_manifest_hash={manifest}");
    println!("no_think={no_think}");
    println!("experimental_temperature_enabled={experimental_temperature}");
    println!("experimental_structured_translation_enabled={experimental_structured_translation}");
    println!("passive_receipt_hash={receipt}");
    println!("lightcone_token_binding={}", atom.binding_attached);
    println!("lightcone_preferred_tokens={preferred_count}");
    println!(
        "lightcone_raw_logit_passes={} lightcone_fires={} lightcone_deflections={}",
        atom.binding_stats.raw_logit_passes, atom.binding_stats.fires, atom.binding_stats.deflections
    );
    println!("lightcone_passive=true lightcone_holds_verdict=false mcp_used=false");
    println!("boundary=weights/raw_logits -> passive_lightcone -> token_selection");
    println!("postweights_executive_dispatch={}", atom.outcome.is_some());
    println!(
        "postweights_sourced_law_dispatch={}",
        atom.sourced_law_report.is_some()
    );
    if let Some(report) = &atom.sourced_law_report {
        println!(
            "sourced_law_strategy={} sourced_law_status={:?} sourced_law_direction={:?} sourced_law_paths={} sourced_law_confidence={:?}",
            report.plan.strategy,
            report.status,
            report.direction,
            report.considered_paths.len(),
            report.confidence
        );
        println!(
            "sourced_law_factual_claim_authority={}",
            report.factual_claim_authority
        );
    }
    println!("lightcone_sourced_law_typed_join=false");
    println!(
        "structured_translation_used={} reasoning_brief_used={}",
        atom.context_kind == "structured-translation",
        atom.context_kind == "reasoning-brief"
    );
    println!(
        "bare_tokens={} bare_postweights_passes={}",
        bare.selected, bare.postweights_passes
    );
    println!(
        "atom_tokens={} atom_postweights_passes={} atom_deflections={}",
        atom.selected, atom.postweights_passes, atom.counterfactual_deflections
    );
    println!("bare_text={:?}", bare.text);
    println!("atom_text={:?}", atom.text);
    println!("factual_claim_authority=false");
    println!("admitted_links={}", boundary_input.admission.links.len());
    println!(
        "out_lane_verdict={:?} out_lane_release={:?} out_lane_reason={}",
        atom_check.verdict, atom_check.release, atom_check.reason
    );
    println!(
        "out_lane_claim={:?} out_lane_claim_polarity={:?} out_lane_claim_class={:?}",
        atom_check.claim, atom_check.claim_polarity, atom_check.claim_class
    );
    println!(
        "out_lane_engine_status={:?} out_lane_engine_direction={:?}",
        atom_check.engine_status, atom_check.engine_direction
    );
    println!(
        "out_lane_asserted={:?} out_lane_negated={:?}",
        atom_check.asserted, atom_check.negated
    );
    println!("out_lane_factual_claim_authority={}", atom_check.factual_claim_authority);
    println!("bare_out_lane_verdict={:?}", bare_check.verdict);
    println!("reinforced={reinforced}");
    if !ledger_note.is_empty() {
        println!("warrant_ledger_note={ledger_note}");
    }
    println!(
        "warrant_ledger_claim={:?} warrant_ledger_reinforcements={} warrant_ledger_claims={} warrant_ledger_total={}",
        atom_check.claim,
        atom_check.claim.as_ref().map_or(0, |claim| ledger.count(claim)),
        ledger.claims(),
        ledger.total_reinforcements()
    );
    println!("warrant_ledger_path={}", ledger_path.display());
    println!("released_text={released:?}");
    println!(
        "membrane_lanes=in_admitted_links:{} postweights_fires:{}/{} out:{:?} feedback:{}",
        boundary_input.admission.links.len(),
        atom.binding_stats.fires,
        atom.binding_stats.raw_logit_passes,
        atom_check.verdict,
        if reinforced { "reinforced" } else { "none" }
    );
    // The executive's answer and the consumers, each quantity on its own line.
    if let Some(outcome) = atom.outcome.as_ref() {
        println!(
            "executive_rung={:?} executive_status={:?} executive_direction={:?} executive_conflict={} winner_declared={}",
            outcome.rung, outcome.status, outcome.direction, outcome.conflict, outcome.winner_declared
        );
        println!(
            "executive_selected_lanes={:?}",
            outcome.selection.lanes
        );
        for (lane, reason) in &outcome.selection.reasons {
            println!("executive_selected={lane} because {reason}");
        }
        for summary in consumers::lane_summaries(&outcome.lanes) {
            println!(
                "lane={:?} status={:?} direction={:?} evidence_weight={:?} paths={} hypotheses={}",
                summary.lane,
                summary.status,
                summary.direction,
                summary.evidence_weight,
                summary.paths,
                summary.hypotheses
            );
        }
        for position in &outcome.positions {
            if position.reason.starts_with("skipped:") {
                println!("lane_skipped={:?} reason={}", position.lane, position.reason);
            }
        }
        println!("contested_evidence={}", consumers::contested_evidence(&outcome.lanes));
        for path in consumers::prediction_paths(&boundary_input.admission, &outcome.lanes) {
            println!(
                "prediction_path={:?} prediction_sources={:?} prediction_sign={:?} prediction_established={}",
                path.link_ids, path.source_ids, path.sign, path.established
            );
        }
        for bridge in &outcome.bridges {
            println!(
                "bridge={:?}->{:?} bridge_relation={:?} bridge_links={:?} bridge_sources={:?} bridge_evidence_class={:?}",
                bridge.from, bridge.to, bridge.relation, bridge.link_ids, bridge.source_ids, bridge.evidence_class
            );
        }
        for path in &outcome.creative {
            println!(
                "creative_path label={:?} status={} kind={} score={:.6} band=({:.3},{:.3}) concepts={:?} spaces={:?} sources={:?} proposal_only={} factual_claim_authority={}",
                path.label, path.status, path.probability_kind, path.probability_score,
                path.uncertainty_interval.0, path.uncertainty_interval.1,
                path.concept_ids, path.spaces, path.source_ids, path.proposal_only,
                path.factual_claim_authority
            );
        }
        if !outcome.creative.is_empty() {
            println!("creative_disclaimer={}", atom_executive::CreativePath::DISCLAIMER);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postweights_policy_uses_same_raw_candidates_and_draw() {
        let mut bare = Selector::new(42, None, None, None, false);
        let mut atom = Selector::new(42, None, None, None, false);
        let logits = [0.0, 1.0, 0.4, -1.0];
        let a = bare.select(&logits).expect("bare");
        let b = atom.select(&logits).expect("atom");
        assert!(a >= 0 && b >= 0);
        assert_eq!(bare.passes, 1);
        assert_eq!(atom.passes, 1);
    }

    #[test]
    fn absent_cognitive_path_is_byte_identical_to_bare_selection() {
        let mut bare = Selector::new(42, None, None, None, false);
        let mut atom = Selector::new(42, None, None, None, false);
        for _ in 0..32 {
            assert_eq!(
                bare.select(&[0.0, 1.0, 0.4]).expect("bare"),
                atom.select(&[0.0, 1.0, 0.4]).expect("atom")
            );
        }
    }
}
