//! The correctness gate: how many cases each arm gets right.
//!
//! Ground truth is the pack's own typed edges, frozen in
//! `fixtures/correctness-flagship.json` (the live instrument, authored on the
//! sealed pack). The development fixture it superseded,
//! `fixtures/correctness-corpus-placeholder-era-2026-09-24b.json`, named the
//! removed placeholder pack's vocabulary and is kept as history only. For every
//! case the runner's JSON is scored by machine, on both arms, with the same
//! instrument:
//!
//! - **connected** (the pack carries an edge from cause to effect): the answer
//!   must name both labels, must assert a relation between them, and must not
//!   deny it. The engine must also have found the path — a retrieval score
//!   separate from the answer score, reported on its own line.
//! - **unconnected** (the pack carries no such edge, and possibly no such
//!   nodes): the engine must find no path, nothing may be reinforced, and the
//!   output lane must not report that the model grounded a claim.
//!
//! The direction instrument is the runner's own marker check (`out_lane_*`),
//! applied identically to both arms, so the score cannot depend on which arm
//! the scorer happens to favour. A case passes only when every check on it
//! passes; the score is the count of passing cases per arm, reported per class
//! as well as in total. Nothing is blended and no check is weighted.
//!
//! usage: correctness <model.gguf> <pack-dir> <corpus.json> [--binary PATH]

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

struct Case {
    id: String,
    truth: String,
    query: String,
    cause: String,
    effect: String,
    goal: String,
    terms: Vec<String>,
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("case needs a {key} string"))
}

fn load(path: &std::path::Path) -> Result<(String, Vec<Case>), String> {
    let bytes = std::fs::read(path).map_err(|error| format!("corpus read failed: {error}"))?;
    let document: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("corpus is not valid JSON: {error}"))?;
    if document.get("schema").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported corpus schema".to_string());
    }
    let revision = text(&document, "revision")?;
    let cases = document
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "corpus needs a cases list".to_string())?
        .iter()
        .map(|case| {
            Ok(Case {
                id: text(case, "id")?,
                truth: text(case, "truth")?,
                query: text(case, "query")?,
                cause: text(case, "cause")?,
                effect: text(case, "effect")?,
                goal: text(case, "goal")?,
                terms: case
                    .get("terms")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                    .unwrap_or_default(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((revision, cases))
}

fn strings(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Every lane path count the runner reported.
fn path_counts(report: &Value) -> Vec<u64> {
    report
        .get("lane_evidence_weights")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(|item| number(item, "paths")).collect())
        .unwrap_or_default()
}

fn mentions(hay: &str, needle: &str) -> bool {
    hay.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
}

/// The checks for one arm of one case. Empty result = the case passes.
fn score_arm(case: &Case, report: &Value, answer: &str, asserted_pointer: &str, negated_pointer: &str, engine: bool) -> Vec<String> {
    let mut failures = Vec::new();
    let asserted = strings(report, asserted_pointer);
    let negated = strings(report, negated_pointer);
    let counts = path_counts(report);
    let found_path = counts.iter().any(|count| *count > 0);

    if case.truth == "connected" {
        // The answer must name both ends of the edge it is being asked about.
        for term in &case.terms {
            if !mentions(answer, term) {
                failures.push(format!("answer does not name {term:?}"));
            }
        }
        if asserted.is_empty() {
            failures.push("answer asserts no relation between them".to_string());
        }
        if !negated.is_empty() {
            failures.push(format!("answer denies the relation via {negated:?}"));
        }
        if engine && !found_path {
            failures.push("no lane found the admitted path".to_string());
        }
    } else {
        // Unconnected: the correct behaviour is to find nothing and to credit
        // nothing to the engine — while still producing an answer, because an
        // empty answer is a production failure even when its content is right.
        if answer.trim().is_empty() {
            failures.push("answer is empty; abstaining on the claim does not license producing nothing".to_string());
        }
        if engine && found_path {
            failures.push("a lane claimed a path the pack does not carry".to_string());
        }
        if asserted.is_empty() && !case.terms.is_empty() {
            // The answer may still discuss the concepts; the check here is that
            // the engine did not ground the claim.
        }
        if report.get("reinforced").and_then(Value::as_bool).unwrap_or(false) {
            failures.push("the harness reinforced a claim from an unconnected pair".to_string());
        }
        let verdict = report
            .get("out_lane_verdict")
            .and_then(Value::as_str)
            .unwrap_or("absent");
        if !matches!(verdict, "NotApplicable" | "Ungrounded") {
            failures.push(format!("output lane reported {verdict} for an unconnected pair"));
        }
    }
    failures
}

fn argument(args: &mut impl Iterator<Item = String>, label: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {label}"))
}

/// FNV-1a 64-bit over the fixture bytes: an integrity key against accidental
/// drift between runs, printed into the receipt. Deliberately labelled
/// non-cryptographic; adversarial tampering is out of scope for a gate that
/// ships alongside its own fixture.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let model = argument(&mut args, "model GGUF")?;
    let pack = argument(&mut args, "Lightcone pack")?;
    let corpus = argument(&mut args, "corpus JSON")?;
    let mut binary = std::env::current_exe()
        .map_err(|error| format!("cannot locate this executable: {error}"))?
        .parent()
        .ok_or("no binary directory")?
        .join("native-harness");
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--binary" => binary = PathBuf::from(argument(&mut args, "--binary value")?),
            "--brief" => {}
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    let brief = std::env::args().any(|flag| flag == "--brief");

    let (revision, cases) = load(std::path::Path::new(&corpus))?;
    let corpus_bytes = std::fs::read(&corpus)
        .map_err(|error| format!("could not re-read the corpus: {error}"))?;
    println!("correctness corpus {revision} — {} cases, both arms, one instrument", cases.len());
    println!("corpus_integrity=fnv1a64:{:016x} bytes:{}", fnv1a64(&corpus_bytes), corpus_bytes.len());
    println!("ground truth is the pack's own edges; scoring is deterministic; no blended score");
    println!("brief: {}", if brief { "on (admitted geometry + lanes in the atom arm's context)" } else { "off" });
    println!("pack: {pack}");

    let mut bare_right = 0usize;
    let mut atom_right = 0usize;
    let mut pack_manifest_hash = String::new();
    for case in &cases {
        let ledger = std::env::temp_dir()
            .join(format!("native-harness-correctness-{}-{}.json", std::process::id(), case.id));
        std::fs::remove_file(&ledger).ok();
        let mut command = Command::new(&binary);
        command
            .arg(&model)
            .arg(&pack)
            .arg(&case.query)
            .arg(&case.cause)
            .arg(&case.effect)
            .arg(&case.goal)
            .arg("128")
            .arg("--json")
            .arg("--ledger")
            .arg(&ledger);
        if brief {
            command.arg("--reasoning-brief");
        }
        let output = command
            .output()
            .map_err(|error| format!("case {} could not run: {error}", case.id))?;
        let report: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "case {} produced no report: {error}; stderr: {}",
                case.id,
                String::from_utf8_lossy(&output.stderr).chars().take(200).collect::<String>()
            )
        })?;
        std::fs::remove_file(&ledger).ok();

        let bare_answer = report.pointer("/bare/text").and_then(Value::as_str).unwrap_or_default();
        let atom_answer = report.pointer("/atom/text").and_then(Value::as_str).unwrap_or_default();
        if pack_manifest_hash.is_empty() {
            if let Some(hash) = report.get("pack_manifest_hash").and_then(Value::as_str) {
                pack_manifest_hash = hash.to_string();
                println!("pack_manifest_hash={hash}");
            }
        }
        let flatten = |text: &str| -> String {
            text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(160).collect()
        };
        let bare_failures = score_arm(case, &report, bare_answer, "/bare_out_lane_asserted", "/bare_out_lane_negated", false);
        let atom_failures = score_arm(case, &report, atom_answer, "/out_lane_asserted", "/out_lane_negated", true);
        let counts = path_counts(&report);
        println!(
            "\n{} [{}] admitted_links={} lane_paths={:?} rung={} creative_paths={}",
            case.id,
            case.truth,
            number(&report, "admitted_links"),
            counts,
            report.get("executive_rung").and_then(Value::as_str).unwrap_or("absent"),
            number(&report, "creative_path_count"),
        );
        println!("  bare_text: {:?}", flatten(bare_answer));
        println!("  atom_text: {:?}", flatten(atom_answer));
        if bare_failures.is_empty() {
            bare_right += 1;
            println!("  bare: CORRECT");
        } else {
            println!("  bare: WRONG — {}", bare_failures.join("; "));
        }
        if atom_failures.is_empty() {
            atom_right += 1;
            println!("  atom: CORRECT");
        } else {
            println!("  atom: WRONG — {}", atom_failures.join("; "));
        }
    }

    let total = cases.len();
    println!("\n--- correctness (the only score) ---");
    println!("bare arm: {bare_right}/{total} correct");
    println!("atom arm: {atom_right}/{total} correct");
    if atom_right > bare_right {
        println!("grounding gained {} case(s)", atom_right - bare_right);
    } else if bare_right > atom_right {
        println!("grounding lost {} case(s)", bare_right - atom_right);
    } else {
        println!("grounding changed nothing on this corpus");
    }
    println!("note: no correctness scoring can license a quality claim by itself — this is a count of cases right, not a measure of how right the answers are");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(truth: &str, terms: &[&str]) -> Case {
        Case {
            id: "c".into(),
            truth: truth.into(),
            query: "q".into(),
            cause: "a".into(),
            effect: "b".into(),
            goal: "causal".into(),
            terms: terms.iter().map(|term| (*term).to_string()).collect(),
        }
    }

    fn report(found_paths: u64, asserted: &[&str], reinforced: bool, verdict: &str) -> Value {
        serde_json::json!({
            "lane_evidence_weights": [{"lane": "Causal", "paths": found_paths}],
            "out_lane_asserted": asserted,
            "out_lane_negated": [],
            "reinforced": reinforced,
            "out_lane_verdict": verdict,
        })
    }

    #[test]
    fn a_connected_case_needs_both_terms_a_relation_and_the_path() {
        let case = case("connected", &["hammer", "nail"]);
        assert!(score_arm(&case, &report(1, &["because"], false, "Consistent"), "A hammer drives a nail because of mass.", "/out_lane_asserted", "/out_lane_negated", true).is_empty());
        let missing_term = score_arm(&case, &report(1, &["because"], false, "Consistent"), "A hammer is heavy.", "/out_lane_asserted", "/out_lane_negated", true);
        assert!(missing_term.iter().any(|f| f.contains("nail")));
        let no_path = score_arm(&case, &report(0, &["because"], false, "Consistent"), "A hammer drives a nail because of mass.", "/out_lane_asserted", "/out_lane_negated", true);
        assert!(no_path.iter().any(|f| f.contains("no lane found")));
    }

    #[test]
    fn an_unconnected_case_needs_silence_from_the_engine() {
        let case = case("unconnected", &[]);
        assert!(score_arm(&case, &report(0, &[], false, "Ungrounded"), "Dough rises as yeast respires.", "/out_lane_asserted", "/out_lane_negated", true).is_empty());
        let claimed = score_arm(&case, &report(1, &[], false, "Ungrounded"), "Dough rises.", "/out_lane_asserted", "/out_lane_negated", true);
        assert!(claimed.iter().any(|f| f.contains("claimed a path")));
        let credited = score_arm(&case, &report(0, &[], true, "Consistent"), "Dough rises.", "/out_lane_asserted", "/out_lane_negated", true);
        assert_eq!(credited.len(), 2, "both the reinforcement and the verdict are wrong: {credited:?}");
    }
}
