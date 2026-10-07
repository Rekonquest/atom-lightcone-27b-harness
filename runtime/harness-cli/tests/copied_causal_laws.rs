use atom_causal_laws::{LawProblem, LawRegistry, LawWorld};
use harness_cli::causal_laws::{copied_world_root, load_copied_worlds};
use atom_orchestrator::{run_laws, Engine, Sign, Status};

fn registry() -> LawRegistry {
    load_copied_worlds(&copied_world_root()).expect("five pinned source worlds")
}

#[test]
fn copied_original_worlds_load_with_distinct_law_semantics() {
    let worlds = registry();
    assert_eq!(worlds.worlds().len(), 5);
    assert_eq!(worlds.worlds().iter().map(|world| world.laws.len()).sum::<usize>(), 652);
    let path = worlds.paths(&LawProblem {
        cause: "philosophy:explanation_phenomenon".into(),
        effect: "philosophy:explanation_explanatory_hypothesis".into(),
        domain: Some("symbolic".into()), conditions: vec![], allowed_domains: vec![],
    }).expect("original direct law path");
    assert!(path.iter().any(|item| item.law_ids.contains(
        &"law:01e07c02cafea26c28d59aac".to_string())));
    assert!(path.iter().all(|item| !item.provenance_hashes.is_empty()));
    assert!(path.iter().all(|item| item.confidence > 0.0 && item.persistence > 0.0));
}

#[test]
fn law_domain_and_unknown_controls_do_not_invent_paths() {
    let worlds = registry();
    let wrong_domain = worlds.paths(&LawProblem {
        cause: "philosophy:explanation_phenomenon".into(),
        effect: "philosophy:explanation_explanatory_hypothesis".into(),
        domain: Some("physical".into()), conditions: vec![], allowed_domains: vec![],
    }).expect("domain filtering");
    assert!(wrong_domain.is_empty());
    let unknown = worlds.paths(&LawProblem {
        cause: "absent".into(), effect: "unknown".into(),
        domain: None, conditions: vec![], allowed_domains: vec![],
    }).expect("unknown filtering");
    assert!(unknown.is_empty());
}

#[test]
fn fixed_orchestrator_invokes_sourced_law_executive() {
    let worlds = registry();
    let decision = run_laws(&Engine, &worlds, &LawProblem {
        cause: "philosophy:explanation_phenomenon".into(),
        effect: "philosophy:explanation_explanatory_hypothesis".into(),
        domain: Some("symbolic".into()), conditions: vec![], allowed_domains: vec![],
    }).expect("sourced law decision");
    assert_eq!(decision.plan.strategy, "fast_causal_recall");
    assert_eq!(decision.status, Status::SupportedPath);
    assert_eq!(decision.direction, Some(Sign::Supports));
    assert!(decision.confidence.is_some_and(|value| value > 0.0));
    assert!(!decision.factual_claim_authority);
    assert!(decision.considered_paths.iter().any(|path|
        path.law_ids.contains(&"law:01e07c02cafea26c28d59aac".to_string())));
}

#[test]
fn cross_world_path_executes_when_no_direct_world_routes() {
    let copied = registry();
    let mut first = copied.worlds()[0].laws[0].clone();
    first.id = "synthetic-one".into();
    first.cause = "A".into();
    first.effect = "B".into();
    first.direction = 1;
    first.provenance_hashes = vec!["synthetic-fixture-one".into()];
    first.evidence_ids = vec!["synthetic-evidence-one".into()];
    let mut second = first.clone();
    second.id = "synthetic-two".into();
    second.cause = "B".into();
    second.effect = "C".into();
    second.direction = -1;
    second.provenance_hashes = vec!["synthetic-fixture-two".into()];
    second.evidence_ids = vec!["synthetic-evidence-two".into()];
    let worlds = LawRegistry::new(vec![
        LawWorld { id: "synthetic-w1".into(), model_hash: "a".repeat(64),
            reliability: 1.0, laws: vec![first] },
        LawWorld { id: "synthetic-w2".into(), model_hash: "b".repeat(64),
            reliability: 1.0, laws: vec![second] },
    ]).expect("synthetic fixture");
    let report = run_laws(&Engine, &worlds, &LawProblem {
        cause: "A".into(), effect: "C".into(), domain: None,
        conditions: vec![], allowed_domains: vec![],
    }).expect("cross-world execution");
    assert_eq!(report.plan.strategy, "active_world_search");
    assert!(report.plan.selected.is_empty());
    assert_eq!(report.status, Status::SupportedPath);
    assert_eq!(report.direction, Some(Sign::Opposes));
    assert_eq!(report.considered_paths.len(), 1);
    assert!(report.considered_paths[0].cross_world);
    assert!(!report.factual_claim_authority);
}
