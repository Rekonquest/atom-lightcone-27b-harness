//! The creative probability response: several speculative paths, each with an
//! uncalibrated feasibility estimate and its own uncertainty band.
//!
//! This is the operator's contracted shape, ported from
//! `atom-harness/contracts/atom-creative-probability-response.json` and
//! `atom_creative_probability.py`, which lived in its own module there too:
//!
//! - several paths, capped at five, no winner declared;
//! - every path labelled
//!   `uncalibrated_probability_of_feasibility_if_prototyped`, never a factual
//!   confidence, with scores independent and not normalized across paths;
//! - a choice records intent and authorizes nothing;
//! - when neither the operator's brief nor the admission yields material, one
//!   labelled speculative fallback stands in, so the result is never empty.
//!
//! Creativity is not a causal-geometric lane. Nothing here claims causal truth,
//! and this module never touches the engine's causal path search.

use atom_orchestrator::{Candidate, Concept, CreativeConfig, CreativeInput, Engine, Error, Input, Operation};

/// A creative path. One of the "several answers": a proposal with its own
/// feasibility estimate, never a factual claim.
#[derive(Clone, Debug)]
pub struct CreativePath {
    pub label: String,
    pub candidate_id: String,
    pub status: &'static str,
    pub probability_kind: &'static str,
    pub probability_score: f64,
    pub uncertainty_interval: (f64, f64),
    pub expected_creative_utility: f64,
    pub required_coverage: f64,
    pub novelty: f64,
    pub value: f64,
    pub hard_valid: bool,
    pub concept_ids: Vec<String>,
    pub spaces: Vec<String>,
    pub source_ids: Vec<String>,
    /// Always false: a path is a proposal, never a factual claim.
    pub factual_claim_authority: bool,
    /// Always true: the path is a proposal only.
    pub proposal_only: bool,
    /// Always true: choosing a path records intent and authorizes nothing.
    pub choice_records_intent_only: bool,
}

impl CreativePath {
    pub const KIND: &'static str = "uncalibrated_probability_of_feasibility_if_prototyped";
    pub const STATUS: &'static str = "speculative_candidate_pending_prototype";
    pub const DISCLAIMER: &'static str =
        "Each score is an uncalibrated estimate of feasibility if prototyped, not a factual \
         confidence score. Scores are independent and do not sum to 100%.";
    /// The contract's ceiling on paths.
    const MAX_PATHS: usize = 5;
}

/// The contracted estimate, kept as it was written in the old harness:
/// `clamp(0.08 + 0.30*coverage + 0.18*value + 0.12*hard_valid + 0.10*grounding
/// + 0.05*testability - 0.08*novelty, 0.05, 0.78)` with
///
/// ```text
/// half_width = clamp(0.16 + 0.14*novelty + 0.08*(1-grounding), 0.16, 0.38)
/// ```
///
/// `grounding` and `testability` are bounded proxies: what this layer can
/// observe about a candidate is the candidate itself.
fn score_path(
    index: usize,
    candidate: &Candidate,
    grounding: bool,
    source_ids: &[String],
) -> CreativePath {
    let grounding_value = if grounding { 1.0 } else { 0.0 };
    let testability = candidate.required_coverage.clamp(0.0, 1.0);
    let score = (0.08
        + 0.30 * candidate.required_coverage
        + 0.18 * candidate.value
        + 0.12 * if candidate.hard_valid { 1.0 } else { 0.0 }
        + 0.10 * grounding_value
        + 0.05 * testability
        - 0.08 * candidate.novelty)
        .clamp(0.05, 0.78);
    let half_width =
        (0.16 + 0.14 * candidate.novelty + 0.08 * (1.0 - grounding_value)).clamp(0.16, 0.38);
    CreativePath {
        label: format!("Creative Path {}", index + 1),
        candidate_id: candidate.concept_ids.join("+"),
        status: CreativePath::STATUS,
        probability_kind: CreativePath::KIND,
        probability_score: score,
        uncertainty_interval: ((score - half_width).max(0.0), (score + half_width).min(1.0)),
        expected_creative_utility: candidate.value,
        required_coverage: candidate.required_coverage,
        novelty: candidate.novelty,
        value: candidate.value,
        hard_valid: candidate.hard_valid,
        concept_ids: candidate.concept_ids.clone(),
        spaces: candidate.spaces.clone(),
        source_ids: source_ids.to_vec(),
        factual_claim_authority: false,
        proposal_only: true,
        choice_records_intent_only: true,
    }
}

/// A bounded creative brief derived from the admission, so the creative lane has
/// material even when the operator supplied none. Nothing is invented: every
/// concept comes from an admitted link. Private: `creative_paths` is the surface.
fn derived_brief(input: &Input) -> Option<CreativeInput> {
    // Concept ids must be unique — two admitted links can share an effect — so
    // the link id is the identity here and the effect is carried as material.
    let mut seen: Vec<&str> = Vec::new();
    let concepts: Vec<Concept> = input
        .admission
        .links
        .iter()
        .filter(|link| {
            if seen.contains(&link.id.as_str()) {
                return false;
            }
            seen.push(link.id.as_str());
            true
        })
        .take(16)
        .map(|link| Concept {
            id: link.id.clone(),
            space: link.domain.clone(),
            attributes: vec![format!("{:?}", link.relation), link.effect.clone()],
            value: 0.5,
            memory: false,
        })
        .collect();
    let seed = concepts.first()?.id.clone();
    Some(CreativeInput {
        reasoning: input.clone(),
        brief: atom_orchestrator::Brief {
            id: format!("derived:{}", input.admission.receipt_id),
            required: vec![input.question.effect.clone()],
            preferred: Vec::new(),
            forbidden_pairs: Vec::new(),
            seed_ids: vec![seed],
            frames: Vec::new(),
            tensions: Vec::new(),
        },
        concepts,
        config: CreativeConfig {
            enabled: [
                Operation::GenerationSelection,
                Operation::ConceptualBlending,
                Operation::HardConstraints,
                Operation::NoveltyValueBalance,
                Operation::ProblemFraming,
                Operation::TensionTransformation,
                Operation::MemoryRecombination,
            ]
            .into_iter()
            .collect(),
            max_blend: 2,
            max_candidates: 32,
            frontier_limit: CreativePath::MAX_PATHS,
        },
    })
}

/// The last rung: a labelled speculative candidate. It claims no fact, no cause
/// and no winner; it exists so the result is never empty.
pub fn speculative_fallback(input: &Input) -> CreativePath {
    CreativePath {
        label: "Speculative fallback".into(),
        candidate_id: format!("speculative:{}", input.admission.receipt_id),
        status: CreativePath::STATUS,
        probability_kind: CreativePath::KIND,
        probability_score: 0.05,
        uncertainty_interval: (0.0, 0.21),
        expected_creative_utility: 0.0,
        required_coverage: 0.0,
        novelty: 0.0,
        value: 0.0,
        hard_valid: false,
        concept_ids: Vec::new(),
        spaces: Vec::new(),
        source_ids: Vec::new(),
        factual_claim_authority: false,
        proposal_only: true,
        choice_records_intent_only: true,
    }
}

/// Run the creative lane and return its paths. An empty result is impossible:
/// when nothing can be generated, the speculative fallback is returned instead,
/// and the flag says which happened.
pub fn creative_paths(
    engine: &Engine,
    input: &Input,
    brief: Option<CreativeInput>,
) -> Result<(Vec<CreativePath>, bool), Error> {
    let prepared = brief.or_else(|| derived_brief(input));
    let Some(brief) = prepared else {
        return Ok((vec![speculative_fallback(input)], true));
    };
    let creative = atom_orchestrator::run_creative(engine, brief)?.creativity;
    let grounding = !creative.grounding_source_ids.is_empty();
    let paths: Vec<CreativePath> = creative
        .frontier
        .iter()
        .take(CreativePath::MAX_PATHS)
        .enumerate()
        .map(|(index, candidate)| {
            score_path(index, candidate, grounding, &creative.grounding_source_ids)
        })
        .collect();
    if paths.is_empty() {
        return Ok((vec![speculative_fallback(input)], true));
    }
    Ok((paths, false))
}
