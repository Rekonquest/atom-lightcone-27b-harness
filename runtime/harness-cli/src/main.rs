//! Local standalone adapter. Lightcone remains a separate passive process.

use atom_orchestrator::*;
use harness_cli::{array, field, goal, required, text};
use std::env;

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let pack_dir = required(&mut args, "verified world-pack directory")?;
    let query = required(&mut args, "Lightcone seed query")?;
    let cause = required(&mut args, "cause node ID or -")?;
    let effect = required(&mut args, "effect node ID")?;
    let task = goal(&required(&mut args, "goal")?)?;
    let removed_link = if task == Goal::AlterPremise {
        Some(required(&mut args, "counterfactual edge ID")?)
    } else {
        None
    };
    let analogy = if task == Goal::Transfer {
        Some(Analogy {
            source_cause: required(&mut args, "source cause")?,
            source_effect: required(&mut args, "source effect")?,
            target_cause: required(&mut args, "target cause")?,
            target_effect: required(&mut args, "target effect")?,
        })
    } else {
        None
    };
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }

    let admitted = harness_cli::admit(
        &pack_dir,
        &query,
        (cause != "-").then_some(cause),
        effect,
        task,
        removed_link,
        analogy,
    )?;
    let document = admitted.document;
    let receipt = field(&document, "receipt")?;
    // The executive is the fixed entrypoint. Dispatching the engine directly
    // here let a refusing lane become a process error on this path while the
    // runner's path degraded to the speculative rung — one contract, two
    // behaviours.
    let outcome = atom_executive::run(&Engine, admitted.input, None)
        .map_err(|e| format!("executive rejected input: {e:?}"))?;
    println!("pack_manifest_hash={}", text(receipt, "manifest_hash")?);
    println!("passive_receipt_hash={}", text(receipt, "receipt_hash")?);
    println!(
        "admitted_nodes={} admitted_edges={} truncated={}",
        array(&document, "nodes")?.len(),
        array(&document, "edges")?.len(),
        field(receipt, "truncated")?
            .as_bool()
            .ok_or("invalid truncated flag")?
    );
    println!("rung={:?} status={:?} creative_paths={}", outcome.rung, outcome.status, outcome.creative.len());
    println!("{outcome:#?}");
    Ok(())
}

fn main() -> Result<(), String> {
    run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use harness_cli::{admission_links, local_lightcone};
    use serde_json::Value;
    use std::path::PathBuf;
    use std::process::Command;

    fn pack() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lightcone/artifacts/worlds/network-security-browser-v0/0.2.0")
    }

    fn admitted(query: &str) -> Value {
        let output = Command::new(local_lightcone())
            .args([
                "admit",
                pack().to_str().expect("pack path"),
                query,
                "--hops",
                "2",
                "--max-nodes",
                "64",
                "--direction",
                "future",
            ])
            .output()
            .expect("local Lightcone");
        assert!(output.status.success());
        serde_json::from_slice(&output.stdout).expect("typed Lightcone response")
    }

    #[test]
    fn sealed_pack_preserves_relation_provenance_and_unknown_strength() {
        // Seeded from the cause, the pack admits its own causal edge
        // (TCP acknowledgment enables retransmission), declared conditions
        // included. Provenance travels; calibration does not exist to travel.
        let document = admitted("TCP acknowledgment");
        let links = admission_links(&document).expect("admission mapping");
        let link = links
            .iter()
            .find(|link| link.id == "nsb.relation.net-ack-retransmit")
            .expect("admitted causal edge");
        assert_eq!(link.world, "network-security-browser-v0");
        assert_eq!(link.cause, "net.tcp-ack");
        assert_eq!(link.effect, "net.retransmission");
        assert_eq!(link.relation, Relation::Enables);
        assert_eq!(link.class, EvidenceClass::Established);
        assert_eq!(link.confidence, None);
        assert_eq!(link.conditions.len(), 1);
        assert_eq!(link.source_ids.len(), 5);
        let report = atom_orchestrator::run(
            &Engine,
            Input {
                admission: Admission {
                    receipt_id: text(&document["receipt"], "receipt_hash").expect("receipt ID"),
                    links,
                },
                question: Question {
                    id: "sealed fixture".into(),
                    goal: Goal::Prove,
                    cause: Some("net.tcp-ack".into()),
                    effect: "net.retransmission".into(),
                    allowed_domains: vec![],
                    conditions: vec![],
                    removed_link: None,
                    analogy: None,
                },
            },
        )
        .expect("Atom path");
        assert_eq!(report.lanes[0].paths.len(), 1);
        assert_eq!(report.lanes[0].paths[0].strength, None);
        assert!(!report.factual_claim_authority);
    }

    #[test]
    fn unknown_seed_admits_no_invented_path() {
        let document = admitted("a phrase absent from this world lexicon");
        assert!(admission_links(&document)
            .expect("empty admission")
            .is_empty());
        assert_eq!(
            document["receipt"]["seed_nodes"].as_array().map(Vec::len),
            Some(0)
        );
    }
}
