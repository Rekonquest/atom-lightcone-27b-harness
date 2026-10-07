//! Per-lane contracts for the seven reasoning lanes.
//!
//! These live in their own test crate so the library crate stays inside the
//! 500-physical-line build contract. Seven separate lanes are a doctrine
//! invariant (`lightcone` skill), so each lane's own behaviour is pinned here
//! rather than asserted in prose: what it needs to conclude, what it refuses to
//! conclude, and that no lane collapses into another.

use atom_reasoning_engine::*;


/// One typed edge with the lanes' fields under control. Sign, class and
/// confidence default to the provisional/supporting case; a test that needs
/// another sets the field on the returned link.
fn link(id: &str, world: &str, cause: &str, effect: &str, relation: Relation) -> Link {
    let (sign, class, confidence) = (Sign::Supports, EvidenceClass::Provisional, None);
    Link {
        id: id.into(),
        world: world.into(),
        cause: cause.into(),
        effect: effect.into(),
        domain: "d".into(),
        target_domain: "d".into(),
        conditions: vec![],
        exceptions: vec![],
        scope: "s".into(),
        target_scope: "s".into(),
        source_ids: vec![format!("{id}-source")],
        relation,
        sign,
        class,
        target_class: class,
        confidence,
    }
}

fn question(goal: Goal, cause: Option<&str>, effect: &str) -> Question {
    Question {
        id: "q".into(),
        goal,
        cause: cause.map(str::to_owned),
        effect: effect.into(),
        allowed_domains: vec![],
        conditions: vec![],
        removed_link: None,
        analogy: None,
    }
}

fn dispatch(goal: Goal, cause: Option<&str>, effect: &str, links: Vec<Link>) -> Report {
    let input = Input {
        admission: Admission {
            receipt_id: "receipt".into(),
            links,
        },
        question: question(goal, cause, effect),
    };
    Engine.dispatch(StackRule::EvidenceFirst, input).expect("dispatch")
}

fn lane_named(report: &Report, lane: Lane) -> &LaneResult {
    report
        .lanes
        .iter()
        .find(|result| result.lane == lane)
        .unwrap_or_else(|| panic!("{lane:?} did not run"))
}

fn provisional(from: &str, to: &str) -> Link {
    link("p", "w1", from, to, Relation::Causes)
}

/// Every goal runs exactly the lanes the ladder plans for it.
#[test]
fn each_goal_selects_its_own_lanes() {
    type Case = (Goal, Vec<Lane>, Option<&'static str>, Vec<Link>);
    let cases: Vec<Case> = vec![
        (Goal::Prove, vec![Lane::Deduction], Some("a"), vec![provisional("a", "b")]),
        (Goal::Generalize, vec![Lane::Induction], Some("a"), vec![provisional("a", "b")]),
        (Goal::Explain, vec![Lane::Abduction], Some("a"), vec![provisional("a", "b")]),
        (Goal::TraceCause, vec![Lane::Causal, Lane::Deduction], Some("a"), vec![provisional("a", "b")]),
        (Goal::Estimate, vec![Lane::Probabilistic], Some("a"), vec![provisional("a", "b")]),
    ];
    for (goal, expected, cause, links) in cases {
        let report = dispatch(goal, cause, "b", links);
        let ran: Vec<Lane> = report.lanes.iter().map(|result| result.lane).collect();
        assert_eq!(ran, expected, "goal {goal:?} planned the wrong lanes");
        assert!(!report.factual_claim_authority);
    }
    // The two lanes that need more than a goal are planned the same way.
    let mut analogy = question(Goal::Transfer, Some("a"), "b");
    analogy.analogy = Some(Analogy {
        source_cause: "a".into(),
        source_effect: "b".into(),
        target_cause: "a".into(),
        target_effect: "b".into(),
    });
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            Input {
                admission: Admission {
                    receipt_id: "receipt".into(),
                    links: vec![provisional("a", "b")],
                },
                question: analogy,
            },
        )
        .expect("transfer dispatch");
    assert_eq!(report.lanes.len(), 1);
    assert_eq!(report.lanes[0].lane, Lane::Analogy);

    let mut counterfactual = question(Goal::AlterPremise, Some("a"), "b");
    counterfactual.removed_link = Some("p".into());
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            Input {
                admission: Admission {
                    receipt_id: "receipt".into(),
                    links: vec![provisional("a", "b")],
                },
                question: counterfactual,
            },
        )
        .expect("alter-premise dispatch");
    assert_eq!(report.lanes.len(), 1);
    assert_eq!(report.lanes[0].lane, Lane::Counterfactual);
}

/// Deduction refuses to conclude without established warrant.
#[test]
fn deduction_needs_established_warrant() {
    let loose = dispatch(Goal::Prove, Some("a"), "b", vec![provisional("a", "b")]);
    let lane = lane_named(&loose, Lane::Deduction);
    assert_eq!(lane.status, Status::Unknown);
    assert_eq!(lane.direction, None);
    assert_eq!(lane.paths.len(), 1, "the path is still reported");

    let firm = dispatch(
        Goal::Prove,
        Some("a"),
        "b",
        vec![{ let mut l = link("e", "w1", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Established; l.target_class = EvidenceClass::Established; l.confidence = None; l }],
    );
    let lane = lane_named(&firm, Lane::Deduction);
    assert_eq!(lane.status, Status::SupportedPath);
    assert_eq!(lane.direction, Some(Sign::Supports));
}

/// Induction needs two independently attributed worlds; one is not enough.
#[test]
fn induction_needs_two_independent_worlds() {
    let one = dispatch(Goal::Generalize, Some("a"), "b", vec![provisional("a", "b")]);
    assert_eq!(lane_named(&one, Lane::Induction).status, Status::Unknown);

    let two = dispatch(
        Goal::Generalize,
        Some("a"),
        "b",
        vec![
            { let mut l = link("p1", "w1", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Provisional; l.target_class = EvidenceClass::Provisional; l.confidence = None; l },
            { let mut l = link("p2", "w2", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Provisional; l.target_class = EvidenceClass::Provisional; l.confidence = None; l },
        ],
    );
    let lane = lane_named(&two, Lane::Induction);
    assert_eq!(lane.status, Status::Provisional);
    assert_eq!(lane.direction, None, "an induction does not assert a direction");
    assert_eq!(lane.paths.len(), 2);
}

/// Abduction lists the rival causes rather than picking one.
#[test]
fn abduction_lists_rival_causes() {
    let report = dispatch(
        Goal::Explain,
        Some("a"),
        "b",
        vec![provisional("a", "b"), { let mut l = link("r", "w1", "c", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Provisional; l.target_class = EvidenceClass::Provisional; l.confidence = None; l }],
    );
    let lane = lane_named(&report, Lane::Abduction);
    assert_eq!(lane.status, Status::Provisional);
    assert_eq!(lane.hypotheses, vec!["a".to_string(), "c".to_string()]);
    assert_eq!(lane.direction, None);
}

/// Analogy is a structural projection and says so; without a mapped edge
/// it is unknown, not a guess.
#[test]
fn analogy_without_a_mapped_edge_stays_unknown() {
    let mut q = question(Goal::Transfer, Some("a"), "b");
    q.analogy = Some(Analogy {
        source_cause: "elsewhere".into(),
        source_effect: "other".into(),
        target_cause: "a".into(),
        target_effect: "b".into(),
    });
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            Input {
                admission: Admission {
                    receipt_id: "receipt".into(),
                    links: vec![provisional("a", "b")],
                },
                question: q,
            },
        )
        .expect("dispatch");
    let lane = &report.lanes[0];
    assert_eq!(lane.status, Status::Unknown);
    assert!(lane.hypotheses.is_empty());
    assert!(lane.reason.contains("unverified"));
}

/// The causal lane reads strength only from established paths, and reports
/// the conflict instead of resolving it silently.
#[test]
fn causal_reports_strength_and_respects_conflict() {
    let strong = dispatch(
        Goal::TraceCause,
        Some("a"),
        "b",
        vec![{ let mut l = link("e", "w1", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Established; l.target_class = EvidenceClass::Established; l.confidence = Some(0.8); l }],
    );
    let lane = lane_named(&strong, Lane::Causal);
    assert_eq!(lane.status, Status::SupportedPath);
    assert_eq!(lane.direction, Some(Sign::Supports));

    let conflicted = dispatch(
        Goal::TraceCause,
        Some("a"),
        "b",
        vec![
            { let mut l = link("e1", "w1", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Established; l.target_class = EvidenceClass::Established; l.confidence = Some(0.8); l },
            { let mut l = link("e2", "w1", "a", "b", Relation::Causes); l.sign = Sign::Opposes; l.class = EvidenceClass::Established; l.target_class = EvidenceClass::Established; l.confidence = Some(0.79); l },
        ],
    );
    let lane = lane_named(&conflicted, Lane::Causal);
    assert_eq!(lane.status, Status::Unknown, "a narrow margin is not a verdict");
    assert_eq!(lane.direction, None);
    assert!(lane.reason.contains("conflict"));
}

/// Counterfactual is a bounded intervention on an admitted link.
#[test]
fn counterfactual_requires_an_admitted_link_and_reports_the_alternate() {
    let mut q = question(Goal::AlterPremise, Some("a"), "b");
    q.removed_link = Some("absent".into());
    let error = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            Input {
                admission: Admission {
                    receipt_id: "receipt".into(),
                    links: vec![provisional("a", "b")],
                },
                question: q,
            },
        )
        .expect_err("an unadmitted link is refused");
    assert_eq!(error, Error::Invalid("counterfactual link absent from admission"));

    let mut q = question(Goal::AlterPremise, Some("a"), "b");
    q.removed_link = Some("p".into());
    let report = Engine
        .dispatch(
            StackRule::EvidenceFirst,
            Input {
                admission: Admission {
                    receipt_id: "receipt".into(),
                    links: vec![provisional("a", "b")],
                },
                question: q,
            },
        )
        .expect("dispatch");
    let lane = &report.lanes[0];
    assert_eq!(lane.status, Status::Provisional, "removing the link changed the path");
    assert!(lane.paths.is_empty(), "the alternate holds no path");
    assert!(lane.reason.contains("not an observed world"));
}

/// Probability appears only when the evidence is contested, and never as a
/// factual confidence.
#[test]
fn probabilistic_reports_a_weight_only_when_contested() {
    let one_sided = dispatch(
        Goal::Estimate,
        Some("a"),
        "b",
        vec![{ let mut l = link("e", "w1", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Provisional; l.target_class = EvidenceClass::Provisional; l.confidence = Some(0.6); l }],
    );
    assert_eq!(lane_named(&one_sided, Lane::Probabilistic).probability, None);

    let contested = dispatch(
        Goal::Estimate,
        Some("a"),
        "b",
        vec![
            { let mut l = link("e1", "w1", "a", "b", Relation::Causes); l.sign = Sign::Supports; l.class = EvidenceClass::Provisional; l.target_class = EvidenceClass::Provisional; l.confidence = Some(0.6); l },
            { let mut l = link("e2", "w1", "a", "b", Relation::Causes); l.sign = Sign::Opposes; l.class = EvidenceClass::Provisional; l.target_class = EvidenceClass::Provisional; l.confidence = Some(0.2); l },
        ],
    );
    let lane = lane_named(&contested, Lane::Probabilistic);
    let weight = lane.probability.expect("contested evidence carries a weight");
    assert!((weight - 0.75).abs() < 1e-9, "weight was {weight}");
    assert_eq!(lane.status, Status::Provisional);
    assert!(lane.reason.contains("not factual confidence"));
}

/// The lanes are separate: the same admission yields a different lane, a
/// different status and a different reason for each goal. Nothing collapses
/// into one retrieval score.
#[test]
fn the_lanes_do_not_collapse() {
    let links = || {
        let mut edge = link("e", "w1", "a", "b", Relation::Causes);
        edge.confidence = Some(0.6);
        vec![edge]
    };
    let deduction = dispatch(Goal::Prove, Some("a"), "b", links());
    let induction = dispatch(Goal::Generalize, Some("a"), "b", links());
    let abduction = dispatch(Goal::Explain, Some("a"), "b", links());
    let causal = dispatch(Goal::TraceCause, Some("a"), "b", links());
    let probabilistic = dispatch(Goal::Estimate, Some("a"), "b", links());

    assert_ne!(
        lane_named(&deduction, Lane::Deduction).reason,
        lane_named(&induction, Lane::Induction).reason
    );
    assert_ne!(
        lane_named(&induction, Lane::Induction).reason,
        lane_named(&abduction, Lane::Abduction).reason
    );
    assert_ne!(
        lane_named(&abduction, Lane::Abduction).reason,
        lane_named(&probabilistic, Lane::Probabilistic).reason
    );
    assert_ne!(
        lane_named(&causal, Lane::Causal).reason,
        lane_named(&probabilistic, Lane::Probabilistic).reason
    );
    // And the same admission gives the lanes different positions.
    assert_eq!(lane_named(&abduction, Lane::Abduction).status, Status::Provisional);
    assert_eq!(lane_named(&deduction, Lane::Deduction).status, Status::Unknown);
    assert_eq!(lane_named(&probabilistic, Lane::Probabilistic).probability, None);
}
