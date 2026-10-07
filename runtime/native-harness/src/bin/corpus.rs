//! The development corpus runner: per-case gates over the whole boundary.
//!
//! It shells out to the built harness binary once per case and checks the
//! reported quantities against that case's frozen expectations. Every case
//! reports separately and the summary counts outcomes per class — no blended
//! score is computed, because a single number would hide which family failed
//! (TRAINING_PROTOCOL discipline).
//!
//! This is a wiring and failure-mode gate. It scores no answer for correctness
//! and licenses no model-performance claim; the held-out corpus and the
//! original-mechanism parity gates remain the operator's to freeze.
//!
//! usage: corpus <model.gguf> <pack-dir> <corpus.json> [--binary PATH]
//!               [--creative BRIEF.json] [--ledger PATH]

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Case {
    id: String,
    class: String,
    query: String,
    cause: String,
    effect: String,
    goal: String,
    budget: String,
    /// The analogy mapping a transfer question needs, as (source cause, source
    /// effect). The runner's own flags are what make these lanes reachable.
    analogy: Option<(String, String)>,
    /// The admitted link an alter-premise question removes.
    removed_link: Option<String>,
    expect: Value,
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("case needs a {key} string"))
}

fn load(path: &Path) -> Result<(String, Vec<Case>), String> {
    let bytes = std::fs::read(path).map_err(|error| format!("corpus read failed: {error}"))?;
    let document: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("corpus is not valid JSON: {error}"))?;
    let schema = document.get("schema").and_then(Value::as_u64).unwrap_or(0);
    if schema != 1 {
        return Err(format!("unsupported corpus schema {schema}"));
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
                class: text(case, "class")?,
                query: text(case, "query")?,
                cause: text(case, "cause")?,
                effect: text(case, "effect")?,
                goal: text(case, "goal")?,
                budget: case
                    .get("budget")
                    .and_then(Value::as_u64)
                    .map_or_else(|| "128".to_string(), |budget| budget.to_string()),
                analogy: case
                    .get("analogy")
                    .and_then(Value::as_array)
                    .and_then(|pair| {
                        (pair.len() == 2).then(|| {
                            (
                                pair[0].as_str().unwrap_or_default().to_string(),
                                pair[1].as_str().unwrap_or_default().to_string(),
                            )
                        })
                    }),
                removed_link: case
                    .get("removed_link")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                expect: case
                    .get("expect")
                    .cloned()
                    .ok_or_else(|| format!("case {} needs expectations", text(case, "id").unwrap_or_default()))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((revision, cases))
}

/// The quantities a case's expectations may check.
struct Observed {
    admitted_links: u64,
    out_lane_verdict: String,
    released: bool,
    reinforced: bool,
    arms_identical: bool,
    lightcone_token_binding: bool,
    lightcone_fires: u64,
    lightcone_deflections: u64,
    creative_operations: u64,
    contested_evidence: bool,
    warrant_ledger_total: u64,
    /// Which reasoning lanes ran, and their reported positions. Each is
    /// reported on its own; no lane is merged with another.
    lanes: Vec<String>,
    lane_status: BTreeMap<String, String>,
    lane_weight_present: BTreeMap<String, bool>,
}

fn number(report: &Value, key: &str) -> u64 {
    report.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn observe(report: &Value) -> Observed {
    let bare = report
        .pointer("/bare/text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let atom = report
        .pointer("/atom/text")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut lanes = Vec::new();
    let mut lane_status = BTreeMap::new();
    let mut lane_weight_present = BTreeMap::new();
    if let Some(entries) = report.get("lane_evidence_weights").and_then(Value::as_array) {
        for entry in entries {
            let Some(name) = entry.get("lane").and_then(Value::as_str) else {
                continue;
            };
            lanes.push(name.to_string());
            if let Some(status) = entry.get("status").and_then(Value::as_str) {
                lane_status.insert(name.to_string(), status.to_string());
            }
            lane_weight_present.insert(
                name.to_string(),
                entry.get("evidence_weight").is_some_and(|weight| !weight.is_null()),
            );
        }
    }
    Observed {
        admitted_links: number(report, "admitted_links"),
        out_lane_verdict: report
            .get("out_lane_verdict")
            .and_then(Value::as_str)
            .unwrap_or("absent")
            .to_string(),
        released: report.get("released").and_then(Value::as_bool).unwrap_or(false),
        reinforced: report.get("reinforced").and_then(Value::as_bool).unwrap_or(false),
        arms_identical: bare == atom,
        lightcone_token_binding: report
            .get("lightcone_token_binding")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        lightcone_fires: number(report, "lightcone_fires"),
        lightcone_deflections: number(report, "lightcone_deflections"),
        creative_operations: number(report, "creative_operations"),
        contested_evidence: report
            .get("contested_evidence")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        warrant_ledger_total: number(report, "warrant_ledger_total"),
        lanes,
        lane_status,
        lane_weight_present,
    }
}

/// Every expectation that does not hold, as one line each. An empty result is
/// a pass; expectations the runner does not recognise are named rather than
/// ignored, so a corpus typo cannot silently weaken a gate.
fn evaluate(expect: &Value, observed: &Observed) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(checks) = expect.as_object() else {
        return vec!["expectations must be an object".to_string()];
    };
    for (key, value) in checks {
        let problem = match key.as_str() {
            "admitted_links_at_least" => value
                .as_u64()
                .filter(|bound| observed.admitted_links >= *bound)
                .is_none()
                .then(|| format!("admitted_links {} < {value}", observed.admitted_links)),
            "admitted_links_at_most" => value
                .as_u64()
                .filter(|bound| observed.admitted_links <= *bound)
                .is_none()
                .then(|| format!("admitted_links {} > {value}", observed.admitted_links)),
            "out_lane_verdict_in" => {
                let allowed: Vec<&str> = value
                    .as_array()
                    .map(|items| items.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                (!allowed.contains(&observed.out_lane_verdict.as_str()))
                    .then(|| format!("out_lane_verdict {} not in {allowed:?}", observed.out_lane_verdict))
            }
            "released" => (value.as_bool() != Some(observed.released))
                .then(|| format!("released {} != {value}", observed.released)),
            "reinforced" => (value.as_bool() != Some(observed.reinforced))
                .then(|| format!("reinforced {} != {value}", observed.reinforced)),
            "arms_identical" => (value.as_bool() != Some(observed.arms_identical))
                .then(|| format!("arms_identical {} != {value}", observed.arms_identical)),
            "lightcone_token_binding" => (value.as_bool() != Some(observed.lightcone_token_binding))
                .then(|| format!("lightcone_token_binding {} != {value}", observed.lightcone_token_binding)),
            "lightcone_fires_at_least" => value
                .as_u64()
                .filter(|bound| observed.lightcone_fires >= *bound)
                .is_none()
                .then(|| format!("lightcone_fires {} < {value}", observed.lightcone_fires)),
            "lightcone_fires_at_most" => value
                .as_u64()
                .filter(|bound| observed.lightcone_fires <= *bound)
                .is_none()
                .then(|| format!("lightcone_fires {} > {value}", observed.lightcone_fires)),
            "lightcone_deflections_at_least" => value
                .as_u64()
                .filter(|bound| observed.lightcone_deflections >= *bound)
                .is_none()
                .then(|| format!("lightcone_deflections {} < {value}", observed.lightcone_deflections)),
            "creative_operations" => value
                .as_u64()
                .filter(|expected| observed.creative_operations == *expected)
                .is_none()
                .then(|| format!("creative_operations {} != {value}", observed.creative_operations)),
            "contested_evidence" => (value.as_bool() != Some(observed.contested_evidence))
                .then(|| format!("contested_evidence {} != {value}", observed.contested_evidence)),
            "warrant_ledger_total" => value
                .as_u64()
                .filter(|expected| observed.warrant_ledger_total == *expected)
                .is_none()
                .then(|| format!("warrant_ledger_total {} != {value}", observed.warrant_ledger_total)),
            "lane_ran" => {
                let name = value.as_str().unwrap_or_default();
                (!observed.lanes.iter().any(|lane| lane == name)).then(|| {
                    format!("lane {name:?} did not run; ran {:?}", observed.lanes)
                })
            }
            "lanes_include" => {
                let wanted: Vec<&str> = value
                    .as_array()
                    .map(|items| items.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let missing: Vec<&str> = wanted
                    .iter()
                    .copied()
                    .filter(|name| !observed.lanes.iter().any(|lane| lane == name))
                    .collect();
                (!missing.is_empty())
                    .then(|| format!("lanes {missing:?} did not run; ran {:?}", observed.lanes))
            }
            "lane_status_in" => {
                if let Some(map) = value.as_object() {
                    let mut problems = Vec::new();
                    for (lane, allowed) in map {
                        let permitted: Vec<&str> = allowed
                            .as_array()
                            .map(|items| items.iter().filter_map(Value::as_str).collect())
                            .unwrap_or_default();
                        match observed.lane_status.get(lane) {
                            None => problems.push(format!("lane {lane} did not run")),
                            Some(status) if !permitted.contains(&status.as_str()) => problems
                                .push(format!("lane {lane} status {status} not in {permitted:?}")),
                            Some(_) => {}
                        }
                    }
                    (!problems.is_empty()).then(|| problems.join("; "))
                } else {
                    Some("lane_status_in must be an object".to_string())
                }
            }
            "lane_weight_absent" => {
                let names: Vec<&str> = value
                    .as_array()
                    .map(|items| items.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let mut problems = Vec::new();
                for name in names {
                    match observed.lane_weight_present.get(name) {
                        None => problems.push(format!("lane {name} did not run")),
                        Some(true) => {
                            problems.push(format!("lane {name} reported a weight, expected none"))
                        }
                        Some(false) => {}
                    }
                }
                (!problems.is_empty()).then(|| problems.join("; "))
            }
            other => Some(format!("unrecognised expectation {other:?}")),
        };
        if let Some(problem) = problem {
            failures.push(problem);
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
    let corpus_path = argument(&mut args, "corpus JSON")?;
    let mut binary = std::env::current_exe()
        .map_err(|error| format!("cannot locate this executable: {error}"))?
        .parent()
        .ok_or("no binary directory")?
        .join("native-harness");
    let mut creative: Option<String> = None;
    let mut ledger: Option<String> = None;
    let mut brief = false;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--binary" => binary = PathBuf::from(argument(&mut args, "--binary value")?),
            "--creative" => creative = Some(argument(&mut args, "--creative value")?),
            "--ledger" => ledger = Some(argument(&mut args, "--ledger value")?),
            "--brief" => brief = true,
            other => return Err(format!("unexpected argument {other:?}")),
        }
    }
    if !binary.exists() {
        return Err(format!("harness binary not found at {}", binary.display()));
    }

    let (revision, cases) = load(Path::new(&corpus_path))?;
    let corpus_bytes = std::fs::read(&corpus_path)
        .map_err(|error| format!("could not re-read the corpus: {error}"))?;
    println!("development corpus {revision} — {} cases", cases.len());
    println!(
        "corpus_integrity=fnv1a64:{:016x} bytes:{} pack:{}",
        fnv1a64(&corpus_bytes),
        corpus_bytes.len(),
        pack
    );
    println!("note: wiring and failure-mode gates only; no correctness scoring, no effectiveness claim");
    println!("brief: {}", if brief { "on (admitted geometry + lanes in the atom arm's context)" } else { "off" });

    let mut passed = 0usize;
    let mut by_class: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    for case in &cases {
        // A per-case ledger keeps gating out of the resident warrant memory and
        // makes the counts deterministic.
        let case_ledger = ledger.clone().unwrap_or_else(|| {
            std::env::temp_dir()
                .join(format!(
                    "native-harness-corpus-{}-{}.json",
                    std::process::id(),
                    case.id
                ))
                .display()
                .to_string()
        });
        std::fs::remove_file(&case_ledger).ok();
        let mut command = Command::new(&binary);
        command
            .arg(&model)
            .arg(&pack)
            .arg(&case.query)
            .arg(&case.cause)
            .arg(&case.effect)
            .arg(&case.goal)
            .arg(&case.budget)
            .arg("--json")
            .arg("--ledger")
            .arg(&case_ledger);
          if let Some(creative_brief) = &creative {
              command.arg("--creative").arg(creative_brief);
          }
          if brief {
              command.arg("--reasoning-brief");
          }
        if let Some(link) = &case.removed_link {
            command.arg("--removed-link").arg(link);
        }
        if let Some((source_cause, source_effect)) = &case.analogy {
            command
                .arg("--analogy")
                .arg(source_cause)
                .arg(source_effect);
        }
        let output = command
            .output()
            .map_err(|error| format!("case {} could not run: {error}", case.id))?;
        let report: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            format!(
                "case {} produced no report: {error}; stderr: {}",
                case.id,
                String::from_utf8_lossy(&output.stderr).chars().take(300).collect::<String>()
            )
        })?;
        let observed = observe(&report);
        let failures = evaluate(&case.expect, &observed);
        let entry = by_class.entry(case.class.clone()).or_insert((0, 0));
        if failures.is_empty() {
            passed += 1;
            entry.0 += 1;
            println!(
                "PASS {} [{}] verdict={} admitted_links={} reinforced={} arms_identical={} fires={} deflections={}",
                case.id,
                case.class,
                observed.out_lane_verdict,
                observed.admitted_links,
                observed.reinforced,
                observed.arms_identical,
                observed.lightcone_fires,
                observed.lightcone_deflections
            );
        } else {
            entry.1 += 1;
            println!("FAIL {} [{}]", case.id, case.class);
            for failure in &failures {
                println!("     {failure}");
            }
        }
        if ledger.is_none() {
            std::fs::remove_file(&case_ledger).ok();
        }
    }
    println!("--- per class (never blended) ---");
    for (class, (passed_in_class, failed)) in &by_class {
        println!("class={class} passed={passed_in_class} failed={failed}");
    }
    println!("total passed={passed} failed={}", cases.len() - passed);
    if passed == cases.len() {
        Ok(())
    } else {
        Err("development corpus gates did not all pass".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed() -> Observed {
        Observed {
            admitted_links: 2,
            out_lane_verdict: "Consistent".into(),
            released: true,
            reinforced: true,
            arms_identical: false,
            lightcone_token_binding: true,
            lightcone_fires: 15,
            lightcone_deflections: 1,
            creative_operations: 7,
            contested_evidence: false,
            warrant_ledger_total: 1,
            lanes: Vec::new(),
            lane_status: BTreeMap::new(),
            lane_weight_present: BTreeMap::new(),
        }
    }

    #[test]
    fn lane_expectations_read_each_lane_separately() {
        let mut report = observed();
        report.lanes = vec!["Causal".into(), "Deduction".into()];
        report.lane_status.insert("Causal".into(), "Unknown".into());
        report.lane_weight_present.insert("Causal".into(), false);
        let expect = serde_json::json!({
            "lane_ran": "Causal",
            "lanes_include": ["Causal", "Deduction"],
            "lane_status_in": {"Causal": ["Unknown", "SupportedPath"]},
            "lane_weight_absent": ["Causal"],
        });
        assert!(evaluate(&expect, &report).is_empty());
    }

    #[test]
    fn a_lane_that_did_not_run_or_leaked_a_weight_fails() {
        let mut quiet = observed();
        quiet.lanes = vec!["Deduction".into()];
        quiet.lane_status.insert("Deduction".into(), "Unknown".into());
        quiet.lane_weight_present.insert("Deduction".into(), true);
        let expect = serde_json::json!({
            "lane_ran": "Probabilistic",
            "lane_weight_absent": ["Deduction"],
            "lane_status_in": {"Probabilistic": ["Unknown"]},
        });
        let failures = evaluate(&expect, &quiet);
        assert_eq!(failures.len(), 3, "each missing lane is named: {failures:?}");
    }

    #[test]
    fn a_grounded_case_passes_its_expectations() {
        let expect = serde_json::json!({
            "admitted_links_at_least": 1,
            "out_lane_verdict_in": ["Consistent"],
            "reinforced": true,
            "arms_identical": false,
            "lightcone_fires_at_least": 1,
            "warrant_ledger_total": 1,
        });
        assert!(evaluate(&expect, &observed()).is_empty());
    }

    #[test]
    fn an_unknown_case_passes_only_when_nothing_fired() {
        let mut quiet = observed();
        quiet.admitted_links = 0;
        quiet.out_lane_verdict = "NotApplicable".into();
        quiet.reinforced = false;
        quiet.arms_identical = true;
        quiet.lightcone_token_binding = false;
        quiet.lightcone_fires = 0;
        quiet.warrant_ledger_total = 0;
        let expect = serde_json::json!({
            "admitted_links_at_most": 0,
            "out_lane_verdict_in": ["NotApplicable"],
            "reinforced": false,
            "arms_identical": true,
            "lightcone_token_binding": false,
            "lightcone_fires_at_most": 0,
            "warrant_ledger_total": 0,
        });
        assert!(evaluate(&expect, &quiet).is_empty());
    }

    #[test]
    fn crossing_verdicts_are_failures_with_named_reasons() {
        let expect = serde_json::json!({
            "out_lane_verdict_in": ["NotApplicable"],
            "reinforced": false,
        });
        let failures = evaluate(&expect, &observed());
        assert_eq!(failures.len(), 2);
        assert!(failures[0].contains("Consistent"));
        assert!(failures[1].contains("reinforced"));
    }

    #[test]
    fn an_unrecognised_expectation_fails_rather_than_being_ignored() {
        let expect = serde_json::json!({ "looks_fine": true });
        let failures = evaluate(&expect, &observed());
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("unrecognised"));
    }
}
