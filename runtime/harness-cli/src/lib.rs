//! Typed adapter from a passive Lightcone receipt to Atom input.

pub mod causal_laws;

use atom_orchestrator::{
    Admission, Analogy, EvidenceClass, Goal, Input, Link, Question, Relation, Sign,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing {key}"))
}

pub fn text(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{key} must be text"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be a list"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} contains non-text"))
        })
        .collect()
}

pub fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    field(value, key)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{key} must be a list"))
}

pub fn required(args: &mut impl Iterator<Item = String>, label: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {label}"))
}

pub fn goal(name: &str) -> Result<Goal, String> {
    match name {
        "prove" => Ok(Goal::Prove),
        "generalize" => Ok(Goal::Generalize),
        "explain" => Ok(Goal::Explain),
        "transfer" => Ok(Goal::Transfer),
        "causal" => Ok(Goal::TraceCause),
        "counterfactual" => Ok(Goal::AlterPremise),
        "estimate" => Ok(Goal::Estimate),
        _ => Err(
            "goal must be prove|generalize|explain|transfer|causal|counterfactual|estimate".into(),
        ),
    }
}

fn class(name: &str) -> Result<EvidenceClass, String> {
    match name {
        "formal" => Ok(EvidenceClass::Formal),
        "established" => Ok(EvidenceClass::Established),
        "provisional" => Ok(EvidenceClass::Provisional),
        "hypothesis" => Ok(EvidenceClass::Hypothesis),
        "contested" => Ok(EvidenceClass::Contested),
        "interpretive" => Ok(EvidenceClass::Interpretive),
        "creative" => Ok(EvidenceClass::Creative),
        "synthetic_control" => Ok(EvidenceClass::SyntheticControl),
        _ => Err(format!("unsupported epistemic class: {name}")),
    }
}

fn relation(name: &str) -> Result<Relation, String> {
    match name {
        "causes" => Ok(Relation::Causes),
        "enables" => Ok(Relation::Enables),
        "supports" => Ok(Relation::Supports),
        "grounds" => Ok(Relation::Grounds),
        "clarifies" => Ok(Relation::Clarifies),
        "qualifies" => Ok(Relation::Qualifies),
        "inhibits" => Ok(Relation::Inhibits),
        "prevents" => Ok(Relation::Prevents),
        "contradicts" => Ok(Relation::Contradicts),
        "undercuts" => Ok(Relation::Undercuts),
        "counterexample" => Ok(Relation::Counterexample),
        "prerequisite" => Ok(Relation::Prerequisite),
        "part_of" => Ok(Relation::PartOf),
        "transforms_to" => Ok(Relation::TransformsTo),
        "analogous_to" => Ok(Relation::AnalogousTo),
        "predicts" => Ok(Relation::Predicts),
        "measures" => Ok(Relation::Measures),
        "implements" => Ok(Relation::Implements),
        _ => Err(format!("unsupported Lightcone relation: {name}")),
    }
}

pub fn local_lightcone() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../lightcone/target/debug/lightcone-cli")
}

pub fn admission_links(document: &Value) -> Result<Vec<Link>, String> {
    let receipt = field(document, "receipt")?;
    let worlds = text(receipt, "world_id")?;
    let nodes = array(document, "nodes")?
        .iter()
        .map(|node| Ok((text(node, "node_id")?, node)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let edges = array(document, "edges")?
        .iter()
        .map(|edge| Ok((text(edge, "edge_id")?, edge)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    array(receipt, "traversed_edges")?
        .iter()
        .map(|step| {
            let id = text(step, "edge_id")?;
            let from_id = text(step, "from")?;
            let to_id = text(step, "to")?;
            let edge = edges
                .get(&id)
                .ok_or_else(|| format!("receipt edge not admitted: {id}"))?;
            let source_id = text(edge, "source_node")?;
            let target_id = text(edge, "target_node")?;
            let forward = from_id == source_id && to_id == target_id;
            let reverse = text(edge, "direction")? == "bidirectional"
                && from_id == target_id
                && to_id == source_id;
            if !forward && !reverse {
                return Err(format!("receipt traversal disagrees with {id}"));
            }
            let source = nodes
                .get(&from_id)
                .ok_or_else(|| format!("admitted node missing: {from_id}"))?;
            let target = nodes
                .get(&to_id)
                .ok_or_else(|| format!("admitted node missing: {to_id}"))?;
            let sign = match field(edge, "polarity")?.as_i64() {
                Some(1) => Sign::Supports,
                Some(-1) => Sign::Opposes,
                Some(0) => Sign::Neutral,
                _ => return Err(format!("invalid polarity: {id}")),
            };
            Ok(Link {
                id,
                world: worlds.clone(),
                cause: from_id,
                effect: to_id,
                domain: text(source, "domain_id")?,
                target_domain: text(target, "domain_id")?,
                conditions: strings(edge, "conditions")?,
                exceptions: strings(edge, "exceptions")?,
                scope: text(source, "scope")?,
                target_scope: text(target, "scope")?,
                source_ids: strings(edge, "source_ids")?,
                relation: relation(&text(edge, "kind")?)?,
                sign,
                class: class(&text(source, "epistemic_class")?)?,
                target_class: class(&text(target, "epistemic_class")?)?,
                confidence: None,
            })
        })
        .collect()
}

/// The grounding vocabulary a passive admission exposes: every admitted node's
/// label plus its aliases. This is vocabulary, not a claim — the caller bounds
/// what it turns into token preferences, and an admission that admitted nothing
/// yields an empty list, which is exact pass-through at the sampling boundary.
pub fn grounding_terms(document: &Value) -> Result<Vec<String>, String> {
    let mut terms = Vec::new();
    for node in array(document, "nodes")? {
        if let Some(label) = node.get("label").and_then(Value::as_str) {
            terms.push(label.to_string());
        }
        if let Some(aliases) = node.get("aliases").and_then(Value::as_array) {
            for alias in aliases.iter().filter_map(Value::as_str) {
                terms.push(alias.to_string());
            }
        }
    }
    terms.sort();
    terms.dedup();
    terms.truncate(64);
    Ok(terms)
}

#[derive(Clone, Debug)]
pub struct AdmittedTurn {
    pub document: Value,
    pub input: Input,
}

pub fn admit(
    pack_dir: &str,
    query: &str,
    cause: Option<String>,
    effect: String,
    task: Goal,
    removed_link: Option<String>,
    analogy: Option<Analogy>,
) -> Result<AdmittedTurn, String> {
    let output = Command::new(local_lightcone())
        .args([
            "admit",
            pack_dir,
            query,
            "--hops",
            "2",
            "--max-nodes",
            "64",
            "--direction",
            "future",
        ])
        .output()
        .map_err(|error| format!("local Lightcone launch failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Lightcone admission failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let document: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid Lightcone admission JSON: {error}"))?;
    let receipt = field(&document, "receipt")?;
    if field(receipt, "passive")?.as_bool() != Some(true)
        || field(receipt, "holds_verdict")?.as_bool() != Some(false)
    {
        return Err("Lightcone receipt crossed the passive boundary".into());
    }
    let links = admission_links(&document)?;
    let receipt_id = text(receipt, "receipt_hash")?;
    let input = Input {
        admission: Admission { receipt_id, links },
        question: Question {
            id: query.into(),
            goal: task,
            cause,
            effect,
            allowed_domains: vec![],
            conditions: vec![],
            removed_link,
            analogy,
        },
    };
    Ok(AdmittedTurn { document, input })
}
