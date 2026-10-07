//! Bounded creative mechanisms. Proposals carry no factual or tool authority.

pub use atom_causal_laws::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Operation {
    GenerationSelection,
    ConceptualBlending,
    HardConstraints,
    NoveltyValueBalance,
    ProblemFraming,
    TensionTransformation,
    MemoryRecombination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreativeError {
    Invalid(&'static str),
    UnknownSeed(String),
    DuplicateConcept(String),
}

#[derive(Clone, Debug)]
pub struct Concept {
    pub id: String,
    pub space: String,
    pub attributes: Vec<String>,
    pub value: f64,
    pub memory: bool,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub id: String,
    pub aliases: Vec<(String, String)>,
    pub bonus_attributes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Tension {
    pub left: String,
    pub right: String,
    pub bridge: String,
}

#[derive(Clone, Debug)]
pub struct Brief {
    pub id: String,
    pub required: Vec<String>,
    pub preferred: Vec<String>,
    pub forbidden_pairs: Vec<(String, String)>,
    pub seed_ids: Vec<String>,
    pub frames: Vec<Frame>,
    pub tensions: Vec<Tension>,
}

#[derive(Clone, Debug)]
pub struct CreativeConfig {
    pub enabled: BTreeSet<Operation>,
    pub max_blend: usize,
    pub max_candidates: usize,
    pub frontier_limit: usize,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub frame_id: String,
    pub concept_ids: Vec<String>,
    pub spaces: Vec<String>,
    pub attributes: Vec<String>,
    pub tension_bridges: Vec<String>,
    pub required_mapping: Vec<(String, String)>,
    pub required_coverage: f64,
    pub novelty: f64,
    pub value: f64,
    pub score: f64,
    pub hard_valid: bool,
}

#[derive(Clone, Debug)]
pub struct CreativeResult {
    pub generated: usize,
    pub admissible: usize,
    pub frontier: Vec<Candidate>,
    pub operations: BTreeSet<Operation>,
    pub grounding_source_ids: Vec<String>,
    pub evaluation_only: bool,
    pub factual_claim_authority: bool,
}

pub fn all_operations() -> Result<CreativeConfig, CreativeError> {
    Ok(CreativeConfig {
        enabled: BTreeSet::from([
            Operation::GenerationSelection,
            Operation::ConceptualBlending,
            Operation::HardConstraints,
            Operation::NoveltyValueBalance,
            Operation::ProblemFraming,
            Operation::TensionTransformation,
            Operation::MemoryRecombination,
        ]),
        max_blend: 3,
        max_candidates: 256,
        frontier_limit: 12,
    })
}

fn validate(
    brief: &Brief,
    concepts: &[Concept],
    config: &CreativeConfig,
) -> Result<(), CreativeError> {
    if brief.id.trim().is_empty() || brief.required.is_empty() {
        return Err(CreativeError::Invalid(
            "brief ID and requirements are required",
        ));
    }
    if concepts.is_empty()
        || concepts.len() > 16
        || config.max_blend == 0
        || config.max_blend > 3
        || config.max_candidates == 0
        || config.max_candidates > 256
        || config.frontier_limit == 0
        || config.frontier_limit > 32
    {
        return Err(CreativeError::Invalid(
            "creative input exceeds bounded limits",
        ));
    }
    let ids: BTreeSet<_> = concepts.iter().map(|item| item.id.as_str()).collect();
    if ids.len() != concepts.len() {
        return Err(CreativeError::DuplicateConcept(
            "concept IDs must be unique".into(),
        ));
    }
    if let Some(unknown) = brief.seed_ids.iter().find(|id| !ids.contains(id.as_str())) {
        return Err(CreativeError::UnknownSeed(unknown.clone()));
    }
    if concepts.iter().any(|item| {
        item.id.trim().is_empty()
            || item.space.trim().is_empty()
            || item.attributes.is_empty()
            || item.attributes.iter().any(|a| a.trim().is_empty())
            || !item.value.is_finite()
            || !(0.0..=1.0).contains(&item.value)
    }) {
        return Err(CreativeError::Invalid(
            "concept has missing or invalid fields",
        ));
    }
    Ok(())
}

fn combinations(
    remaining: &[Concept],
    chosen: &[Concept],
    maximum: usize,
) -> Result<Vec<Vec<Concept>>, CreativeError> {
    let here = if chosen.is_empty() {
        vec![]
    } else {
        vec![chosen.to_vec()]
    };
    if chosen.len() >= maximum {
        return Ok(here);
    }
    let below = remaining
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let next: Vec<Concept> = chosen.iter().cloned().chain([item.clone()]).collect();
            combinations(&remaining[index + 1..], &next, maximum)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(here
        .into_iter()
        .chain(below.into_iter().flatten())
        .collect())
}

fn candidate(
    frame: &Frame,
    brief: &Brief,
    combination: &[Concept],
    memory: &[BTreeSet<String>],
    config: &CreativeConfig,
) -> Result<Candidate, CreativeError> {
    let spaces: BTreeSet<String> = combination.iter().map(|c| c.space.clone()).collect();
    let base: BTreeSet<String> = combination
        .iter()
        .flat_map(|c| c.attributes.iter().cloned())
        .collect();
    let bridges: Vec<String> = if config.enabled.contains(&Operation::TensionTransformation) {
        brief
            .tensions
            .iter()
            .filter(|t| base.contains(&t.left) && base.contains(&t.right))
            .map(|t| t.bridge.clone())
            .collect()
    } else {
        vec![]
    };
    let attributes: BTreeSet<String> = base.into_iter().chain(bridges.iter().cloned()).collect();
    let required_mapping: Vec<(String, String)> = brief
        .required
        .iter()
        .map(|original| {
            let mapped = if config.enabled.contains(&Operation::ProblemFraming) {
                frame
                    .aliases
                    .iter()
                    .find(|(from, _)| from == original)
                    .map(|(_, to)| to.as_str())
            } else {
                None
            };
            (
                original.clone(),
                mapped.map_or_else(|| original.clone(), str::to_owned),
            )
        })
        .collect();
    let required: BTreeSet<_> = required_mapping
        .iter()
        .map(|(_, mapped)| mapped.clone())
        .collect();
    let coverage = attributes.intersection(&required).count() as f64 / required.len() as f64;
    let preferred: BTreeSet<_> = brief.preferred.iter().cloned().collect();
    let preference = if preferred.is_empty() {
        1.0
    } else {
        attributes.intersection(&preferred).count() as f64 / preferred.len() as f64
    };
    let bonus = if frame.bonus_attributes.is_empty() {
        0.0
    } else {
        frame
            .bonus_attributes
            .iter()
            .filter(|a| attributes.contains(*a))
            .count() as f64
            / frame.bonus_attributes.len() as f64
    };
    let source_value = combination.iter().map(|c| c.value).sum::<f64>() / combination.len() as f64;
    let value = (0.62 * coverage + 0.16 * preference + 0.10 * bonus + 0.12 * source_value).min(1.0);
    let similarity = memory
        .iter()
        .map(|seen| {
            let union = attributes.union(seen).count();
            if union == 0 {
                0.0
            } else {
                attributes.intersection(seen).count() as f64 / union as f64
            }
        })
        .fold(0.0_f64, f64::max);
    let novelty = 1.0 - similarity;
    let hard_valid = !brief
        .forbidden_pairs
        .iter()
        .any(|(left, right)| attributes.contains(left) && attributes.contains(right));
    let score = if config.enabled.contains(&Operation::NoveltyValueBalance) {
        0.45 * novelty + 0.55 * value
    } else {
        value
    };
    Ok(Candidate {
        frame_id: frame.id.clone(),
        concept_ids: combination.iter().map(|c| c.id.clone()).collect(),
        spaces: spaces.into_iter().collect(),
        attributes: attributes.into_iter().collect(),
        tension_bridges: bridges,
        required_mapping,
        required_coverage: coverage,
        novelty,
        value,
        score,
        hard_valid,
    })
}

pub fn explore(
    brief: &Brief,
    concepts: &[Concept],
    grounding: &[PathTrace],
    config: &CreativeConfig,
) -> Result<CreativeResult, CreativeError> {
    validate(brief, concepts, config)?;
    let available: Vec<Concept> = concepts
        .iter()
        .filter(|c| {
            config.enabled.contains(&Operation::MemoryRecombination)
                || !c.memory
                || brief.seed_ids.contains(&c.id)
        })
        .cloned()
        .collect();
    if available.is_empty() {
        return Err(CreativeError::Invalid("no candidate material"));
    }
    let maximum = if config.enabled.contains(&Operation::ConceptualBlending) {
        config.max_blend
    } else {
        1
    };
    let combinations: Vec<Vec<Concept>> = combinations(&available, &[], maximum)?
        .into_iter()
        .filter(|set| {
            set.len() == 1 || set.iter().map(|c| &c.space).collect::<BTreeSet<_>>().len() > 1
        })
        .take(config.max_candidates)
        .collect();
    let frames = if config.enabled.contains(&Operation::ProblemFraming) && !brief.frames.is_empty()
    {
        brief.frames.clone()
    } else {
        vec![Frame {
            id: "literal".into(),
            aliases: vec![],
            bonus_attributes: vec![],
        }]
    };
    let memory: Vec<BTreeSet<String>> = concepts
        .iter()
        .filter(|c| c.memory)
        .map(|c| c.attributes.iter().cloned().collect())
        .collect();
    let generated = frames
        .iter()
        .flat_map(|frame| {
            combinations
                .iter()
                .map(|set| candidate(frame, brief, set, &memory, config))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let admissible: Vec<Candidate> = generated
        .iter()
        .filter(|c| !config.enabled.contains(&Operation::HardConstraints) || c.hard_valid)
        .cloned()
        .collect();
    let frontier: Vec<Candidate> = if config.enabled.contains(&Operation::GenerationSelection) {
        admissible
            .iter()
            .filter(|candidate| {
                !admissible.iter().any(|rival| {
                    rival.score >= candidate.score
                        && rival.required_coverage >= candidate.required_coverage
                        && (rival.score > candidate.score
                            || rival.required_coverage > candidate.required_coverage)
                })
            })
            .take(config.frontier_limit)
            .cloned()
            .collect()
    } else {
        admissible.iter().take(1).cloned().collect()
    };
    let sources: BTreeSet<String> = grounding
        .iter()
        .flat_map(|path| path.source_ids.iter().cloned())
        .collect();
    Ok(CreativeResult {
        generated: generated.len(),
        admissible: admissible.len(),
        frontier,
        operations: config.enabled.clone(),
        grounding_source_ids: sources.into_iter().collect(),
        evaluation_only: config.enabled.len() != 7,
        factual_claim_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concepts() -> Vec<Concept> {
        vec![
            Concept {
                id: "a".into(),
                space: "one".into(),
                attributes: vec!["left".into()],
                value: 0.7,
                memory: false,
            },
            Concept {
                id: "b".into(),
                space: "two".into(),
                attributes: vec!["right".into()],
                value: 0.8,
                memory: true,
            },
        ]
    }

    fn brief() -> Brief {
        Brief {
            id: "fixture".into(),
            required: vec!["need".into()],
            preferred: vec![],
            forbidden_pairs: vec![],
            seed_ids: vec!["a".into()],
            frames: vec![Frame {
                id: "reframe".into(),
                aliases: vec![("need".into(), "bridge".into())],
                bonus_attributes: vec![],
            }],
            tensions: vec![Tension {
                left: "left".into(),
                right: "right".into(),
                bridge: "bridge".into(),
            }],
        }
    }

    #[test]
    fn seven_mechanisms_produce_traceable_constrained_proposals() {
        let result = explore(
            &brief(),
            &concepts(),
            &[],
            &all_operations().expect("config"),
        )
        .expect("proposals");
        assert_eq!(result.operations.len(), 7);
        assert!(result.frontier.iter().any(|c| c.concept_ids == ["a", "b"]
            && c.spaces == ["one", "two"]
            && c.attributes.contains(&"bridge".into())
            && c.required_coverage == 1.0));
        assert!(!result.factual_claim_authority);
        assert!(!result.evaluation_only);
    }

    #[test]
    fn hard_constraints_outrank_novelty_even_with_memory() {
        let brief = Brief {
            forbidden_pairs: vec![("left".into(), "right".into())],
            ..brief()
        };
        let safe = explore(&brief, &concepts(), &[], &all_operations().expect("config"))
            .expect("safe proposals");
        assert!(safe.frontier.iter().all(|c| c.hard_valid));
        let config = all_operations().expect("config");
        let evaluation = CreativeConfig {
            enabled: config
                .enabled
                .iter()
                .copied()
                .filter(|op| *op != Operation::HardConstraints)
                .collect(),
            ..config
        };
        let unsafe_arm = explore(&brief, &concepts(), &[], &evaluation).expect("ablation");
        assert!(unsafe_arm.evaluation_only);
        assert!(unsafe_arm.frontier.iter().any(|c| !c.hard_valid));
        assert!(!unsafe_arm.factual_claim_authority);
    }
}
