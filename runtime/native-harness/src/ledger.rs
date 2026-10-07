//! Bounded warrant memory for verified claims — host-side, because it persists.
//!
//! Verified-claim feedback, the fourth lane: a claim the grounded output used
//! in its own direction is reinforced here and survives a restart. The ledger
//! records *usage*, never truth. The claim's epistemic class is stored beside
//! its count rather than merged with it, no score is computed from it, and
//! nothing here is fed back into the model as a fact.
//!
//! Only the grounded arm reinforces. The bare arm's text may happen to use the
//! same claim out of the model's own knowledge, and crediting the engine for
//! that would be exactly the kind of unearned uplift this harness refuses.

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const LEDGER_SCHEMA: u64 = 1;
/// Bounded like every other store in this harness; a full ledger refuses new
/// claims instead of silently evicting old ones.
pub const LEDGER_LIMIT: usize = 1024;

#[derive(Clone, Debug)]
pub struct WarrantEntry {
    pub claim_id: String,
    pub world: String,
    pub polarity: String,
    pub claim_class: String,
    pub reinforcements: u64,
    pub last_receipt: String,
}

#[derive(Clone, Debug, Default)]
pub struct WarrantLedger {
    entries: BTreeMap<String, WarrantEntry>,
}

impl WarrantLedger {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(format!("warrant ledger read failed: {error}")),
        };
        let document: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("warrant ledger is not valid JSON: {error}"))?;
        let schema = document.get("schema").and_then(Value::as_u64).unwrap_or(0);
        if schema != LEDGER_SCHEMA {
            return Err(format!("warrant ledger schema {schema} is not supported"));
        }
        let mut entries = BTreeMap::new();
        if let Some(map) = document.get("entries").and_then(Value::as_object) {
            for (claim_id, entry) in map {
                let text = |key: &str| {
                    entry
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                entries.insert(
                    claim_id.clone(),
                    WarrantEntry {
                        claim_id: claim_id.clone(),
                        world: text("world"),
                        polarity: text("polarity"),
                        claim_class: text("claim_class"),
                        reinforcements: entry
                            .get("reinforcements")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        last_receipt: text("last_receipt"),
                    },
                );
            }
        }
        Ok(Self { entries })
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("warrant ledger directory failed: {error}"))?;
        }
        let mut map = Map::new();
        for (claim_id, entry) in &self.entries {
            map.insert(
                claim_id.clone(),
                json!({
                    "world": entry.world,
                    "polarity": entry.polarity,
                    "claim_class": entry.claim_class,
                    "reinforcements": entry.reinforcements,
                    "last_receipt": entry.last_receipt,
                }),
            );
        }
        let document = json!({ "schema": LEDGER_SCHEMA, "entries": map });
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("warrant ledger serialization failed: {error}"))?;
        std::fs::write(path, bytes).map_err(|error| format!("warrant ledger write failed: {error}"))
    }

    /// Reinforce one verified claim. Returns the entry's new count, or the
    /// reason the write was refused.
    pub fn reinforce(&mut self, entry: WarrantEntry) -> Result<u64, String> {
        if !self.entries.contains_key(&entry.claim_id) && self.entries.len() >= LEDGER_LIMIT {
            return Err(format!("warrant ledger is full at {LEDGER_LIMIT} claims"));
        }
        let stored = self.entries.entry(entry.claim_id.clone()).or_insert(WarrantEntry {
            reinforcements: 0,
            ..entry.clone()
        });
        stored.reinforcements += 1;
        stored.world = entry.world;
        stored.polarity = entry.polarity;
        stored.claim_class = entry.claim_class;
        stored.last_receipt = entry.last_receipt;
        Ok(stored.reinforcements)
    }

    pub fn count(&self, claim_id: &str) -> u64 {
        self.entries.get(claim_id).map_or(0, |entry| entry.reinforcements)
    }

    pub fn total_reinforcements(&self) -> u64 {
        self.entries.values().map(|entry| entry.reinforcements).sum()
    }

    pub fn claims(&self) -> usize {
        self.entries.len()
    }
}

/// The harness root's runtime store, pinned like every other harness path.
pub fn default_ledger_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.runtime/lightcone-boundary/verdicts.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!(
            "native-harness-ledger-{}-{}-{name}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        );
        std::env::temp_dir().join(unique)
    }

    fn entry(claim: &str) -> WarrantEntry {
        WarrantEntry {
            claim_id: claim.into(),
            world: "network-security-browser-v0".into(),
            polarity: "Supports".into(),
            claim_class: "Established".into(),
            reinforcements: 0,
            last_receipt: "ac121fb3".into(),
        }
    }

    #[test]
    fn reinforcement_survives_a_restart() {
        let path = temp_path("restart");
        let mut ledger = WarrantLedger::load(&path).expect("empty ledger");
        assert_eq!(ledger.reinforce(entry("nsb.relation.net-ack-retransmit")).unwrap(), 1);
        ledger.save(&path).expect("save");

        // A fresh load is what a restarted process sees.
        let mut restarted = WarrantLedger::load(&path).expect("reload");
        assert_eq!(restarted.count("nsb.relation.net-ack-retransmit"), 1);
        assert_eq!(restarted.reinforce(entry("nsb.relation.net-ack-retransmit")).unwrap(), 2);
        assert_eq!(restarted.total_reinforcements(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_missing_ledger_is_empty_not_an_error() {
        let path = temp_path("missing");
        let ledger = WarrantLedger::load(&path).expect("missing file is empty");
        assert_eq!(ledger.claims(), 0);
        assert_eq!(ledger.total_reinforcements(), 0);
    }

    #[test]
    fn an_unknown_schema_is_refused() {
        let path = temp_path("schema");
        std::fs::write(&path, br#"{"schema": 99, "entries": {}}"#).expect("write");
        let error = WarrantLedger::load(&path).expect_err("schema refused");
        assert!(error.contains("schema 99"), "got: {error}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_full_ledger_refuses_new_claims_without_evicting() {
        let mut ledger = WarrantLedger::default();
        for index in 0..LEDGER_LIMIT {
            ledger.reinforce(entry(&format!("claim.{index}"))).expect("within limit");
        }
        let error = ledger.reinforce(entry("claim.one-too-many")).expect_err("full");
        assert!(error.contains("full"), "got: {error}");
        assert_eq!(ledger.claims(), LEDGER_LIMIT, "nothing was evicted");
        assert_eq!(ledger.count("claim.0"), 1, "existing entries survive");
    }

    #[test]
    fn the_class_is_stored_beside_the_count_not_merged_into_it() {
        let mut ledger = WarrantLedger::default();
        ledger.reinforce(entry("nsb.relation.net-ack-retransmit")).expect("reinforced");
        let stored = ledger.entries.get("nsb.relation.net-ack-retransmit").expect("entry");
        assert_eq!(stored.claim_class, "Established");
        assert_eq!(stored.reinforcements, 1);
    }
}
