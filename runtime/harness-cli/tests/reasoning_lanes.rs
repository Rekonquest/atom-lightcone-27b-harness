use atom_orchestrator::*;

fn link(
    id: &str,
    world: &str,
    cause: &str,
    effect: &str,
    source: &str,
    sign: Sign,
    confidence: f64,
) -> Link {
    Link {
        id: id.into(),
        world: world.into(),
        cause: cause.into(),
        effect: effect.into(),
        domain: "physical".into(),
        target_domain: "physical".into(),
        conditions: vec!["condition:general".into()],
        exceptions: vec![],
        scope: "synthetic-test".into(),
        target_scope: "synthetic-test".into(),
        source_ids: vec![source.into()],
        relation: Relation::Causes,
        sign,
        class: EvidenceClass::Established,
        target_class: EvidenceClass::Established,
        confidence: Some(confidence),
    }
}

fn input(goal: Goal, cause: Option<&str>, effect: &str, links: Vec<Link>) -> Input {
    Input {
        admission: Admission {
            receipt_id: "synthetic-fixture-only".into(),
            links,
        },
        question: Question {
            id: "fixture".into(),
            goal,
            cause: cause.map(str::to_owned),
            effect: effect.into(),
            allowed_domains: vec![],
            conditions: vec![],
            removed_link: None,
            analogy: None,
        },
    }
}

#[test]
fn cross_world_path_runs_without_direct_world_selection() {
    let links = vec![
        link("a", "world-1", "A", "B", "source-a", Sign::Supports, 0.9),
        link("b", "world-2", "B", "C", "source-b", Sign::Supports, 0.8),
    ];
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::TraceCause, Some("A"), "C", links),
        )
        .expect("typed path");
    assert_eq!(report.lanes.len(), 2);
    assert_eq!(report.lanes[0].status, Status::SupportedPath);
    assert_eq!(report.lanes[0].paths[0].world_ids, ["world-1", "world-2"]);
    assert_eq!(
        report.lanes[0].paths[0].source_ids,
        ["source-a", "source-b"]
    );
    assert!(!report.factual_claim_authority);
}

#[test]
fn signed_conflict_abstains_and_negative_path_is_not_flipped() {
    let conflict = vec![
        link("yes", "w1", "A", "B", "s1", Sign::Supports, 0.8),
        link("no", "w2", "A", "B", "s2", Sign::Opposes, 0.75),
    ];
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::TraceCause, Some("A"), "B", conflict),
        )
        .expect("conflict");
    assert_eq!(report.lanes[0].status, Status::Unknown);
    assert_eq!(report.lanes[1].status, Status::Unknown);
    let negative = vec![
        link("yes", "w1", "A", "B", "s1", Sign::Supports, 0.1),
        link("no", "w2", "A", "B", "s2", Sign::Opposes, 0.9),
    ];
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::TraceCause, Some("A"), "B", negative),
        )
        .expect("negative path");
    assert_eq!(report.lanes[0].direction, Some(Sign::Opposes));
}

#[test]
fn all_seven_lanes_execute_distinct_contracts() {
    let links = vec![
        link("one", "w1", "A", "B", "s1", Sign::Supports, 0.6),
        link("two", "w2", "A", "B", "s2", Sign::Supports, 0.8),
        link("rival", "w3", "X", "B", "s3", Sign::Opposes, 0.3),
        link("against", "w4", "A", "B", "s4", Sign::Opposes, 0.4),
    ];
    let run = |goal, cause, effect, laws| {
        Engine
            .dispatch(StackRule::EvidenceFirst, input(goal, cause, effect, laws))
            .expect("lane execution")
            .lanes[0]
            .clone()
    };
    assert_eq!(
        run(Goal::Prove, Some("A"), "B", links.clone()).lane,
        Lane::Deduction
    );
    assert_eq!(
        run(Goal::Generalize, Some("A"), "B", links.clone()).status,
        Status::Provisional
    );
    let abductive = run(Goal::Explain, None, "B", links.clone());
    assert_eq!(abductive.lane, Lane::Abduction);
    assert_eq!(abductive.hypotheses, ["A", "X"]);
    let analogy = Input {
        question: Question {
            goal: Goal::Transfer,
            analogy: Some(Analogy {
                source_cause: "A".into(),
                source_effect: "B".into(),
                target_cause: "C".into(),
                target_effect: "D".into(),
            }),
            ..input(Goal::Transfer, None, "D", vec![]).question
        },
        ..input(Goal::Transfer, None, "D", links.clone())
    };
    let analogy = Engine
        .dispatch(StackRule::EvidenceFirst, analogy)
        .expect("analogy");
    assert_eq!(analogy.lanes[0].status, Status::Provisional);
    let counterfactual = Input {
        question: Question {
            removed_link: Some("one".into()),
            ..input(Goal::AlterPremise, Some("A"), "B", vec![]).question
        },
        ..input(Goal::AlterPremise, Some("A"), "B", vec![links[0].clone()])
    };
    let counterfactual = Engine
        .dispatch(StackRule::EvidenceFirst, counterfactual)
        .expect("counterfactual");
    assert_eq!(counterfactual.lanes[0].status, Status::Provisional);
    assert_eq!(
        run(Goal::TraceCause, Some("A"), "B", links.clone()).lane,
        Lane::Causal
    );
    let probability = run(Goal::Estimate, Some("A"), "B", links);
    assert_eq!(probability.lane, Lane::Probabilistic);
    assert_eq!(probability.status, Status::Provisional);
}

#[test]
fn conditions_evidence_class_and_invalid_admission_fail_closed() {
    let mut scoped = link("one", "w1", "A", "B", "s1", Sign::Supports, 0.8);
    scoped.conditions = vec!["condition:hot".into()];
    let mut cold = input(Goal::TraceCause, Some("A"), "B", vec![scoped.clone()]);
    cold.question.conditions = vec!["condition:cold".into()];
    assert_eq!(
        Engine
            .dispatch(StackRule::EvidenceFirst, cold)
            .expect("cold")
            .lanes[0]
            .status,
        Status::Unknown
    );
    scoped.class = EvidenceClass::Provisional;
    let provisional = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::TraceCause, Some("A"), "B", vec![scoped]),
        )
        .expect("provisional");
    assert_eq!(provisional.lanes[0].status, Status::Unknown);
    let invalid = input(
        Goal::Prove,
        Some("A"),
        "B",
        vec![
            link("same", "w1", "A", "B", "s1", Sign::Supports, 0.8),
            link("same", "w2", "A", "B", "s2", Sign::Supports, 0.7),
        ],
    );
    assert_eq!(
        Engine.dispatch(StackRule::EvidenceFirst, invalid).err(),
        Some(Error::DuplicateLink("same".into()))
    );
}

#[test]
fn lightcone_geometry_does_not_gain_invented_confidence_or_causal_type() {
    let mut edge = link("geometry", "w", "A", "B", "source", Sign::Supports, 0.9);
    edge.confidence = None;
    edge.class = EvidenceClass::Provisional;
    let causal = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::TraceCause, Some("A"), "B", vec![edge.clone()]),
        )
        .expect("geometry");
    assert_eq!(causal.lanes[0].status, Status::Unknown);
    assert_eq!(causal.lanes[0].paths[0].strength, None);
    let chance = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::Estimate, Some("A"), "B", vec![edge.clone()]),
        )
        .expect("probability");
    assert_eq!(chance.lanes[0].probability, None);
    edge.confidence = Some(0.9);
    edge.class = EvidenceClass::Established;
    edge.relation = Relation::Supports;
    let typed = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::TraceCause, Some("A"), "B", vec![edge]),
        )
        .expect("typed relation");
    assert_eq!(typed.lanes[0].status, Status::Unknown);
    assert_eq!(typed.lanes[1].status, Status::SupportedPath);
}

#[test]
fn many_sources_on_one_edge_are_not_independent_worlds_or_a_probability() {
    let mut edge = link("only", "single-world", "A", "B", "s1", Sign::Supports, 0.8);
    edge.source_ids = vec!["s1".into(), "s2".into(), "s3".into()];
    let induction = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::Generalize, Some("A"), "B", vec![edge.clone()]),
        )
        .expect("single-world observation");
    assert_eq!(induction.lanes[0].status, Status::Unknown);
    let probability = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            input(Goal::Estimate, Some("A"), "B", vec![edge]),
        )
        .expect("unopposed evidence");
    assert_eq!(probability.lanes[0].probability, None);
}
