//! Creativity is a proposal lane. It generates traceable, constrained
//! candidates through bottom-up composition and lateral reframing, but never
//! grants factual, causal, safety, or permission authority to novelty.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Concept {
    pub concept_id: String,
    pub space: String,
    pub attributes: BTreeSet<String>,
    pub utility: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub frame_id: String,
    pub added_preferences: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Tension {
    pub left: String,
    pub right: String,
    pub bridge: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Brief {
    pub brief_id: String,
    pub required: BTreeSet<String>,
    pub preferences: BTreeSet<String>,
    pub forbidden_pairs: BTreeSet<(String, String)>,
    pub frames: Vec<Frame>,
    pub tensions: Vec<Tension>,
    pub max_blend_size: usize,
    pub max_candidates: usize,
    pub novelty_weight: f64,
    pub value_weight: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub candidate_id: String,
    pub source_concepts: Vec<String>,
    pub source_spaces: Vec<String>,
    pub frame_id: Option<String>,
    pub attributes: Vec<String>,
    pub tension_bridges: Vec<String>,
    pub novelty: f64,
    pub value: f64,
    pub score: f64,
    pub hard_constraints_satisfied: bool,
    pub proposal_only: bool,
    pub factual_authority: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreativeResult {
    pub brief_id: String,
    pub candidates_considered: usize,
    pub candidates_rejected_by_constraints: usize,
    pub selected: Option<Candidate>,
    pub frontier: Vec<Candidate>,
    pub bottom_up_trace: Vec<String>,
    pub lateral_frames_considered: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProposalRequest {
    pub brief: Brief,
    pub concepts: Vec<Concept>,
}

#[derive(Clone, Debug)]
pub struct CreativityEngine;

impl CreativityEngine {
    pub fn propose(
        &self,
        brief: &Brief,
        concepts: &[Concept],
    ) -> Result<CreativeResult, CreativityError> {
        validate(brief, concepts)?;
        let combinations = combinations(concepts.len(), brief.max_blend_size, brief.max_candidates);
        let memory_sets = concepts
            .iter()
            .map(|concept| concept.attributes.clone())
            .collect::<Vec<_>>();
        let mut generated = Vec::new();
        let mut rejected = 0usize;
        let mut trace = Vec::new();
        let frames = std::iter::once(None)
            .chain(brief.frames.iter().map(Some))
            .collect::<Vec<_>>();

        for indices in combinations {
            let selected = indices
                .iter()
                .map(|index| &concepts[*index])
                .collect::<Vec<_>>();
            let source_ids = selected
                .iter()
                .map(|concept| concept.concept_id.clone())
                .collect::<Vec<_>>();
            let spaces = selected
                .iter()
                .map(|concept| concept.space.clone())
                .collect::<BTreeSet<_>>();
            let base_attributes = selected
                .iter()
                .flat_map(|concept| concept.attributes.iter().cloned())
                .collect::<BTreeSet<_>>();
            trace.push(format!(
                "compose:{}=>{}",
                source_ids.join("+"),
                base_attributes
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("+")
            ));

            for frame in &frames {
                if generated.len() >= brief.max_candidates {
                    break;
                }
                let mut attributes = base_attributes.clone();
                if let Some(frame) = frame {
                    attributes.extend(frame.added_preferences.iter().cloned());
                }
                let mut bridges = Vec::new();
                for tension in &brief.tensions {
                    if attributes.contains(&tension.left) && attributes.contains(&tension.right) {
                        attributes.insert(tension.bridge.clone());
                        bridges.push(tension.bridge.clone());
                    }
                }
                let hard_valid = brief.required.is_subset(&attributes)
                    && brief.forbidden_pairs.iter().all(|(left, right)| {
                        !(attributes.contains(left) && attributes.contains(right))
                    });
                if !hard_valid {
                    rejected += 1;
                    continue;
                }
                let novelty = novelty(&attributes, &memory_sets, spaces.len());
                let preference_fit = if brief.preferences.is_empty() {
                    1.0
                } else {
                    brief.preferences.intersection(&attributes).count() as f64
                        / brief.preferences.len() as f64
                };
                let source_value = selected.iter().map(|concept| concept.utility).sum::<f64>()
                    / selected.len() as f64;
                let value = (preference_fit + source_value) / 2.0;
                let score = brief.novelty_weight * novelty + brief.value_weight * value;
                let frame_suffix = frame.map(|frame| frame.frame_id.as_str()).unwrap_or("base");
                generated.push(Candidate {
                    candidate_id: format!(
                        "{}:{}:{frame_suffix}",
                        brief.brief_id,
                        source_ids.join("+")
                    ),
                    source_concepts: source_ids.clone(),
                    source_spaces: spaces.iter().cloned().collect(),
                    frame_id: frame.map(|frame| frame.frame_id.clone()),
                    attributes: attributes.into_iter().collect(),
                    tension_bridges: bridges,
                    novelty,
                    value,
                    score,
                    hard_constraints_satisfied: true,
                    proposal_only: true,
                    factual_authority: false,
                });
            }
        }
        generated.sort_by(candidate_order);
        let selected = generated.first().cloned();
        let frontier = pareto_front(&generated);
        Ok(CreativeResult {
            brief_id: brief.brief_id.clone(),
            candidates_considered: generated.len() + rejected,
            candidates_rejected_by_constraints: rejected,
            selected,
            frontier,
            bottom_up_trace: trace,
            lateral_frames_considered: brief
                .frames
                .iter()
                .map(|frame| frame.frame_id.clone())
                .collect(),
        })
    }
}

fn validate(brief: &Brief, concepts: &[Concept]) -> Result<(), CreativityError> {
    if brief.brief_id.trim().is_empty() || brief.required.is_empty() {
        return Err(CreativityError::Invalid(
            "brief ID and required attributes must be non-empty".into(),
        ));
    }
    if !(1..=4).contains(&brief.max_blend_size) || !(1..=4096).contains(&brief.max_candidates) {
        return Err(CreativityError::Invalid(
            "blend size must be 1..=4 and candidate limit 1..=4096".into(),
        ));
    }
    if !brief.novelty_weight.is_finite()
        || !brief.value_weight.is_finite()
        || brief.novelty_weight < 0.0
        || brief.value_weight < 0.0
        || (brief.novelty_weight + brief.value_weight - 1.0).abs() > 1e-9
    {
        return Err(CreativityError::Invalid(
            "finite non-negative novelty/value weights must sum to one".into(),
        ));
    }
    if concepts.is_empty() {
        return Err(CreativityError::Invalid("concept memory is empty".into()));
    }
    let mut ids = BTreeSet::new();
    for concept in concepts {
        if concept.concept_id.trim().is_empty()
            || concept.space.trim().is_empty()
            || concept.attributes.is_empty()
            || !concept.utility.is_finite()
            || !(0.0..=1.0).contains(&concept.utility)
            || !ids.insert(&concept.concept_id)
        {
            return Err(CreativityError::Invalid(format!(
                "invalid or duplicate concept {}",
                concept.concept_id
            )));
        }
    }
    for (left, right) in &brief.forbidden_pairs {
        if left == right || left.trim().is_empty() || right.trim().is_empty() {
            return Err(CreativityError::Invalid(
                "forbidden pairs require two distinct attributes".into(),
            ));
        }
    }
    Ok(())
}

fn combinations(count: usize, max_size: usize, limit: usize) -> Vec<Vec<usize>> {
    fn visit(
        count: usize,
        target: usize,
        start: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
        limit: usize,
    ) {
        if output.len() >= limit {
            return;
        }
        if current.len() == target {
            output.push(current.clone());
            return;
        }
        for index in start..count {
            current.push(index);
            visit(count, target, index + 1, current, output, limit);
            current.pop();
            if output.len() >= limit {
                return;
            }
        }
    }
    let mut output = Vec::new();
    for size in 1..=max_size.min(count) {
        visit(count, size, 0, &mut Vec::new(), &mut output, limit);
        if output.len() >= limit {
            break;
        }
    }
    output
}

fn novelty(attributes: &BTreeSet<String>, memory: &[BTreeSet<String>], spaces: usize) -> f64 {
    let nearest = memory
        .iter()
        .map(|known| {
            let union = attributes.union(known).count();
            if union == 0 {
                1.0
            } else {
                attributes.intersection(known).count() as f64 / union as f64
            }
        })
        .fold(0.0_f64, f64::max);
    (1.0 - nearest + 0.08 * spaces.saturating_sub(1) as f64).min(1.0)
}

fn candidate_order(left: &Candidate, right: &Candidate) -> Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| right.value.total_cmp(&left.value))
        .then_with(|| right.novelty.total_cmp(&left.novelty))
        .then_with(|| left.candidate_id.cmp(&right.candidate_id))
}

fn pareto_front(candidates: &[Candidate]) -> Vec<Candidate> {
    let mut output = candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other.candidate_id != candidate.candidate_id
                    && other.novelty >= candidate.novelty
                    && other.value >= candidate.value
                    && (other.novelty > candidate.novelty || other.value > candidate.value)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    output.sort_by(candidate_order);
    output
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CreativityError {
    #[error("invalid creativity input: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn concepts() -> Vec<Concept> {
        vec![
            Concept {
                concept_id: "quiet-room".into(),
                space: "architecture".into(),
                attributes: set(&["quiet", "shelter", "private"]),
                utility: 0.8,
            },
            Concept {
                concept_id: "garden".into(),
                space: "ecology".into(),
                attributes: set(&["living", "open", "restorative"]),
                utility: 0.7,
            },
            Concept {
                concept_id: "library".into(),
                space: "learning".into(),
                attributes: set(&["quiet", "shared", "knowledge"]),
                utility: 0.9,
            },
        ]
    }

    fn brief() -> Brief {
        Brief {
            brief_id: "rest-space".into(),
            required: set(&["quiet", "restorative"]),
            preferences: set(&["living", "knowledge"]),
            forbidden_pairs: [("private".into(), "shared".into())].into_iter().collect(),
            frames: vec![Frame {
                frame_id: "learning-as-rest".into(),
                added_preferences: set(&["knowledge"]),
            }],
            tensions: vec![Tension {
                left: "private".into(),
                right: "open".into(),
                bridge: "graduated-boundary".into(),
            }],
            max_blend_size: 3,
            max_candidates: 64,
            novelty_weight: 0.45,
            value_weight: 0.55,
        }
    }

    #[test]
    fn hard_constraints_are_never_traded_for_novelty() {
        let result = CreativityEngine.propose(&brief(), &concepts()).unwrap();
        assert!(result.candidates_rejected_by_constraints > 0);
        for candidate in result.frontier {
            assert!(candidate.hard_constraints_satisfied);
            let attributes = candidate.attributes.into_iter().collect::<BTreeSet<_>>();
            assert!(!(attributes.contains("private") && attributes.contains("shared")));
        }
    }

    #[test]
    fn candidate_provenance_and_truth_boundary_are_explicit() {
        let result = CreativityEngine.propose(&brief(), &concepts()).unwrap();
        let selected = result.selected.unwrap();
        assert!(!selected.source_concepts.is_empty());
        assert!(!selected.source_spaces.is_empty());
        assert!(selected.proposal_only);
        assert!(!selected.factual_authority);
    }

    #[test]
    fn tension_requires_a_declared_bridge() {
        let result = CreativityEngine.propose(&brief(), &concepts()).unwrap();
        let bridged = result.frontier.iter().find(|candidate| {
            candidate.attributes.contains(&"private".into())
                && candidate.attributes.contains(&"open".into())
        });
        assert!(bridged.is_some_and(|candidate| {
            candidate
                .tension_bridges
                .contains(&"graduated-boundary".into())
        }));
    }

    #[test]
    fn bottom_up_trace_and_lateral_frames_are_retained() {
        let result = CreativityEngine.propose(&brief(), &concepts()).unwrap();
        assert!(
            result
                .bottom_up_trace
                .iter()
                .any(|line| line.contains("quiet-room+garden"))
        );
        assert_eq!(result.lateral_frames_considered, ["learning-as-rest"]);
    }

    #[test]
    fn generation_is_deterministic() {
        let engine = CreativityEngine;
        let first = engine.propose(&brief(), &concepts()).unwrap();
        let second = engine.propose(&brief(), &concepts()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_weights_fail_closed() {
        let mut invalid = brief();
        invalid.novelty_weight = 1.0;
        assert!(CreativityEngine.propose(&invalid, &concepts()).is_err());
    }
}
