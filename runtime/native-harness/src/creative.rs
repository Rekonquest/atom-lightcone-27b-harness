//! Host side of the creative dispatch: a bounded creative brief loaded from a
//! file, combined with the reasoning input the admission produced.
//!
//! The brief is host-supplied material — required and preferred attributes,
//! forbidden pairs, seed concepts, frames, tensions. Nothing here invents
//! concepts, and a malformed or over-bounded file is refused rather than
//! clamped: the engine's own `validate` remains the authority.

use atom_orchestrator::{Brief, Concept, CreativeConfig, CreativeInput, Frame, Input, Tension};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

fn text(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("creative brief needs a {key} string"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("creative brief needs a {key} list"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} must contain only strings"))
        })
        .collect()
}

fn pairs(value: &Value, key: &str) -> Result<Vec<(String, String)>, String> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("creative brief needs a {key} list"))?;
    items
        .iter()
        .map(|item| {
            let pair = item
                .as_array()
                .filter(|pair| pair.len() == 2)
                .ok_or_else(|| format!("{key} entries must be two-element lists"))?;
            let left = pair[0]
                .as_str()
                .ok_or_else(|| format!("{key} entries must be strings"))?;
            let right = pair[1]
                .as_str()
                .ok_or_else(|| format!("{key} entries must be strings"))?;
            Ok((left.to_owned(), right.to_owned()))
        })
        .collect()
}

fn parse_brief(value: &Value) -> Result<Brief, String> {
    let brief = value
        .get("brief")
        .ok_or_else(|| "creative brief needs a brief object".to_string())?;
    // Frames and tensions belong to the brief, so they live inside it.
    let frames = brief
        .get("frames")
        .and_then(Value::as_array)
        .map(|frames| {
            frames
                .iter()
                .map(|frame| {
                    Ok(Frame {
                        id: text(frame, "id")?,
                        aliases: pairs(frame, "aliases")?,
                        bonus_attributes: strings(frame, "bonus_attributes")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    let tensions = brief
        .get("tensions")
        .and_then(Value::as_array)
        .map(|tensions| {
            tensions
                .iter()
                .map(|tension| {
                    Ok(Tension {
                        left: text(tension, "left")?,
                        right: text(tension, "right")?,
                        bridge: text(tension, "bridge")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(Brief {
        id: text(brief, "id")?,
        required: strings(brief, "required")?,
        preferred: strings(brief, "preferred").unwrap_or_default(),
        forbidden_pairs: pairs(brief, "forbidden_pairs").unwrap_or_default(),
        seed_ids: strings(brief, "seed_ids").unwrap_or_default(),
        frames,
        tensions,
    })
}

fn parse_concepts(value: &Value) -> Result<Vec<Concept>, String> {
    value
        .get("concepts")
        .and_then(Value::as_array)
        .ok_or_else(|| "creative brief needs a concepts list".to_string())?
        .iter()
        .map(|concept| {
            Ok(Concept {
                id: text(concept, "id")?,
                space: text(concept, "space")?,
                attributes: strings(concept, "attributes")?,
                value: concept
                    .get("value")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| "concept needs a numeric value".to_string())?,
                memory: concept.get("memory").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn number(value: &Value, key: &str, default: usize) -> Result<usize, String> {
    match value.get(key).and_then(Value::as_u64) {
        Some(raw) => usize::try_from(raw).map_err(|_| format!("{key} is out of range")),
        None => Ok(default),
    }
}

/// Load a bounded creative brief and pair it with the reasoning input.
pub fn load(path: &Path, reasoning: Input) -> Result<CreativeInput, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("creative brief read failed: {error}"))?;
    let document: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("creative brief is not valid JSON: {error}"))?;
    let brief = parse_brief(&document)?;
    let concepts = parse_concepts(&document)?;
    let config = CreativeConfig {
        enabled: BTreeSet::from([
            atom_orchestrator::Operation::GenerationSelection,
            atom_orchestrator::Operation::ConceptualBlending,
            atom_orchestrator::Operation::HardConstraints,
            atom_orchestrator::Operation::NoveltyValueBalance,
            atom_orchestrator::Operation::ProblemFraming,
            atom_orchestrator::Operation::TensionTransformation,
            atom_orchestrator::Operation::MemoryRecombination,
        ]),
        max_blend: number(&document, "max_blend", 3)?,
        max_candidates: number(&document, "max_candidates", 64)?,
        frontier_limit: number(&document, "frontier_limit", 8)?,
    };
    Ok(CreativeInput {
        reasoning,
        brief,
        concepts,
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use atom_orchestrator::{Admission, Goal, Question};

    fn reasoning() -> Input {
        Input {
            admission: Admission {
                receipt_id: "r".into(),
                links: vec![],
            },
            question: Question {
                id: "q".into(),
                goal: Goal::Prove,
                cause: Some("a".into()),
                effect: "b".into(),
                allowed_domains: vec![],
                conditions: vec![],
                removed_link: None,
                analogy: None,
            },
        }
    }

    fn write(name: &str, body: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "native-harness-brief-{}-{name}.json",
            std::process::id()
        ));
        std::fs::write(&path, body).expect("write brief");
        path
    }

    fn valid_body() -> String {
        r#"{
          "brief": {
            "id": "fixture",
            "required": ["need"],
            "preferred": [],
            "forbidden_pairs": [],
            "seed_ids": ["a"],
            "frames": [{"id": "reframe", "aliases": [["need","bridge"]], "bonus_attributes": []}],
            "tensions": [{"left": "left", "right": "right", "bridge": "bridge"}]
          },
          "concepts": [{"id": "a", "space": "one", "attributes": ["left", "right"], "value": 0.7}],
          "max_blend": 3, "max_candidates": 32, "frontier_limit": 4
        }"#
        .to_string()
    }

    #[test]
    fn a_valid_brief_loads_with_all_seven_operations() {
        let path = write("valid", &valid_body());
        let input = load(&path, reasoning()).expect("loads");
        assert_eq!(input.brief.id, "fixture");
        assert_eq!(input.concepts.len(), 1);
        assert_eq!(input.config.enabled.len(), 7);
        assert_eq!(input.config.frontier_limit, 4);
        // Frames and tensions must reach the engine, not silently default:
        // the bridge attribute only appears if the tension was parsed.
        assert_eq!(input.brief.frames.len(), 1, "frame reached the brief");
        assert_eq!(input.brief.tensions.len(), 1, "tension reached the brief");
        let result = atom_orchestrator::run_creative(&atom_orchestrator::Engine, input)
            .expect("creative dispatch");
        assert!(result.creativity.generated > 0);
        assert!(
            result
                .creativity
                .frontier
                .iter()
                .any(|candidate| candidate.tension_bridges == ["bridge"]),
            "the parsed tension produced its bridge: {:?}",
            result.creativity.frontier
        );
        assert!(!result.creativity.factual_claim_authority);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_concept_without_a_numeric_value_is_refused() {
        // Swapped for a string rather than removed, so the fixture stays
        // valid JSON and the refusal comes from the value check itself.
        let body = valid_body().replace(r#""value": 0.7"#, r#""value": "high""#);
        let path = write("novalue", &body);
        assert!(load(&path, reasoning()).expect_err("refused").contains("value"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_missing_required_list_is_refused() {
        let body = r#"{"brief": {"id": "x"}, "concepts": []}"#;
        let path = write("norequirements", body);
        assert!(load(&path, reasoning()).expect_err("refused").contains("required"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn over_bounded_config_is_left_to_the_engine_validator() {
        // The host does not clamp; the engine's validate() rejects it.
        let body = valid_body().replace(r#""max_blend": 3"#, r#""max_blend": 9"#);
        let path = write("overbound", &body);
        let input = load(&path, reasoning()).expect("parses");
        assert!(atom_orchestrator::run_creative(&atom_orchestrator::Engine, input).is_err());
        std::fs::remove_file(&path).ok();
    }
}
