//! Read-only loader for the explicitly copied original CausalLaw models.

use atom_causal_laws::{CausalLaw, LawRegistry, LawWorld};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::process::Command;

const PINNED: [(&str, &str, &str, &str); 5] = [
    (
        "algebra-word-reasoning-v1",
        "world_packs/algebra-word-reasoning-v1/atom_algebra_word_world_model.json",
        "10395fc7c4cd7c828504509e8ffcd2c00ad53b7a6e4d9c85de27e8295f43c1e8",
        "3ecab0d2f06d994457c50cec3d7c7c604e3e3729d641d3446dae6ce666d14dfd",
    ),
    (
        "broad-coding-foundation-v1",
        "world_packs/broad-coding-foundation-v1/atom_broad_coding_world_model.json",
        "7e3e795baf4db8122b7c72867d3c72ff8f6344e394cebfe0eeaa2f7af0c97f66",
        "17fc40139bef61f360c4755cb892cf9045f40eb12bd85cb81ad3f4a066958205",
    ),
    (
        "philosophy-reasoning-foundation-v1",
        "world_packs/philosophy-reasoning-foundation-v1/atom_philosophy_world_model.json",
        "0b7de23a4e08ccbc5decfa17bd4aecc91a8f0f71b9d082dfbb6fa9fa5fc4f79c",
        "f0f3d739c8533ee801828e573791acfb1ee0143e91d71c19bab41c276140b2b8",
    ),
    (
        "psychology-foundation-v1",
        "world_packs/psychology-foundation-v1/atom_psychology_world_model.json",
        "58f3a1315451e1c52301b48b0010d4659318da212a3707304f9b09c12e37add9",
        "33f940c487ceee9cea8fc9808d24820ad75a05143f0a06539c65aa13df168fbd",
    ),
    (
        "safe-tool-use-v1",
        "world_packs/safe-tool-use-v1/atom_tool_use_world_model.json",
        "ea07ed4ed4527851006a2d35f9d343284e7241ced5d4a614f2454c8fa9cdcf6d",
        "8d5635d7f3f1722c571025544355b0f18c37494375f5108d978948833cb83df0",
    ),
];

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("causal law missing {key}"))
}

fn string(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("causal law {key} is not text"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("causal law {key} is not finite"))
}

fn count(value: &Value, key: &str) -> Result<u64, String> {
    field(value, key)?
        .as_u64()
        .ok_or_else(|| format!("causal law {key} is not a count"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} is not a list"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} contains non-text"))
        })
        .collect()
}

fn law(value: &Value) -> Result<CausalLaw, String> {
    let direction = field(value, "direction")?
        .as_i64()
        .and_then(|n| i8::try_from(n).ok())
        .ok_or("invalid law direction")?;
    let contexts = field(value, "contexts")?
        .as_object()
        .ok_or("law contexts are not an object")?
        .iter()
        .map(|(key, count)| {
            count
                .as_u64()
                .map(|n| (key.clone(), n))
                .ok_or_else(|| format!("context {key} has invalid count"))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let law = CausalLaw {
        id: string(value, "law_id")?,
        domain: string(value, "domain")?,
        cause: string(value, "cause_feature")?,
        effect: string(value, "effect_feature")?,
        direction,
        magnitude_mean: number(value, "magnitude_mean")?,
        magnitude_m2: number(value, "magnitude_m2")?,
        effect_variance_mean: number(value, "effect_variance_mean")?,
        delay_mean: number(value, "delay_mean")?,
        invariant_error_mean: number(value, "invariant_error_mean")?,
        support: count(value, "support")?,
        contradictions: count(value, "contradictions")?,
        treated_worlds: count(value, "treated_worlds")?,
        baseline_worlds: count(value, "baseline_worlds")?,
        confidence: number(value, "confidence")?,
        persistence: number(value, "persistence")?,
        phase: number(value, "phase")?,
        status: string(value, "status")?,
        atom_program: strings(value, "atom_program")?,
        source_law_ids: strings(value, "source_law_ids")?,
        contexts,
        provenance_hashes: strings(value, "provenance_hashes")?,
        evidence_ids: strings(value, "evidence_ids")?,
    };
    law.validate()
        .map_err(|error| format!("invalid causal law: {error:?}"))?;
    Ok(law)
}

fn file_hash(path: &Path) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|error| format!("SHA-256 invocation failed: {error}"))?;
    if !output.status.success() {
        return Err("SHA-256 invocation failed".into());
    }
    let line = String::from_utf8(output.stdout).map_err(|_| "invalid SHA-256 output")?;
    line.split_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| "empty SHA-256 output".into())
}

pub fn copied_world_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference-causal-worlds")
}

pub fn load_copied_worlds(root: &Path) -> Result<LawRegistry, String> {
    let mut worlds = Vec::new();
    for (world_id, relative, expected_file, expected_model) in PINNED {
        let relative = Path::new(relative);
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err("unsafe causal world path".into());
        }
        let path = root.join(relative);
        if file_hash(&path)? != expected_file {
            return Err(format!(
                "causal world file hash mismatch: {}",
                path.display()
            ));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("causal world read failed: {e}"))?;
        let document: Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("causal world JSON invalid: {e}"))?;
        if string(&document, "model_hash")? != expected_model {
            return Err(format!("causal model hash mismatch: {world_id}"));
        }
        let laws = field(field(&document, "graph")?, "laws")?
            .as_array()
            .ok_or("causal laws are not a list")?
            .iter()
            .map(law)
            .collect::<Result<Vec<_>, _>>()?;
        worlds.push(LawWorld {
            id: world_id.into(),
            model_hash: expected_model.into(),
            reliability: 1.0,
            laws,
        });
    }
    LawRegistry::new(worlds).map_err(|error| format!("causal registry invalid: {error:?}"))
}
