//! Executive contract tests — `docs/EXECUTIVE.md` pinned.
//!
//! These live in a verification leaf crate so every production crate stays
//! inside the build contract's 500 physical lines including its tests.

use atom_executive::{Executive, Outcome, Rung};
use atom_orchestrator::{
    Admission, Analogy, Engine, EvidenceClass, Goal, Input, Lane, Link, Question, Relation, Sign,
    Status,
};

fn link(
    id: &str,
    from: &str,
    to: &str,
    relation: Relation,
    sign: Sign,
    class: EvidenceClass,
    confidence: Option<f64>,
) -> Link {
    Link {
        id: id.into(),
        world: "w1".into(),
        cause: from.into(),
        effect: to.into(),
        domain: "creative-writing".into(),
        target_domain: "creative-writing".into(),
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

fn input(links: Vec<Link>, question: Question) -> Input {
    Input {
        admission: Admission {
            receipt_id: "receipt-1".into(),
            links,
        },
        question,
    }
}

fn established(from: &str, to: &str, relation: Relation, sign: Sign) -> Link {
    link("e1", from, to, relation, sign, EvidenceClass::Established, Some(0.5))
}

fn run(input: Input) -> Outcome {
    Executive::execute(&Engine, input, None).expect("executive")
}

#[test]
fn selection_is_deterministic_multi_lane_and_auditable() {
    let admission = input(
        vec![established("a", "b", Relation::Causes, Sign::Supports)],
        question(Goal::TraceCause, Some("a"), "b"),
    );
    let first = Executive::select(&admission);
    let second = Executive::select(&admission);
    assert_eq!(first.lanes, second.lanes, "selection must be deterministic");
    assert!(first.lanes.contains(&Lane::Causal), "the goal's lane is present");
    assert!(
        first.lanes.contains(&Lane::Deduction),
        "causal tracing plans deduction beside it"
    );
    assert!(
        first.lanes.len() >= 2,
        "selection is never argmax-only: {:?}",
        first.lanes
    );
    assert_eq!(first.reasons.len(), first.lanes.len(), "every lane carries its reason");
}

#[test]
fn selection_adds_lanes_from_typed_features() {
    let mut q = question(Goal::Prove, Some("a"), "b");
    q.analogy = Some(Analogy {
        source_cause: "a".into(),
        source_effect: "b".into(),
        target_cause: "a".into(),
        target_effect: "b".into(),
    });
    q.removed_link = Some("e1".into());
    let mut first = established("a", "b", Relation::Causes, Sign::Supports);
    first.world = "w1".into();
    let mut second_link = established("a", "b", Relation::Predicts, Sign::Opposes);
    second_link.id = "e2".into();
    second_link.world = "w2".into();
    let selection = Executive::select(&input(vec![first, second_link], q));
    for expected in [Lane::Analogy, Lane::Counterfactual, Lane::Induction, Lane::Probabilistic] {
        assert!(
            selection.lanes.contains(&expected),
            "{expected:?} should be selected: {:?}",
            selection.lanes
        );
    }
}

#[test]
fn a_lane_without_its_precondition_is_skipped_with_a_reason() {
    let outcome = run(input(
        vec![established("a", "b", Relation::Causes, Sign::Supports)],
        question(Goal::Transfer, Some("a"), "b"),
    ));
    let analogy = outcome
        .positions
        .iter()
        .find(|position| position.lane == Lane::Analogy)
        .expect("analogy is selected by the goal");
    assert_eq!(analogy.status, Status::Unknown);
    assert!(
        analogy.reason.starts_with("skipped:"),
        "the skip must say why: {}",
        analogy.reason
    );
}

#[test]
fn an_established_path_is_authoritative() {
    let outcome = run(input(
        vec![established("a", "b", Relation::Causes, Sign::Supports)],
        question(Goal::Prove, Some("a"), "b"),
    ));
    assert_eq!(outcome.status, Status::SupportedPath);
    assert_eq!(outcome.direction, Some(Sign::Supports));
    assert_eq!(outcome.rung, Rung::Slow, "a supported path is the slow, authoritative pass");
    assert!(outcome.creative.is_empty(), "reasoning answered; creativity stays quiet");
    assert!(!outcome.conflict);
    assert!(!outcome.factual_claim_authority);
}

#[test]
fn conflicting_lanes_fail_closed_and_say_so() {
    // Deduction can see the supporting relation, the causal lane can see the
    // inhibiting one. Neither is wrong; the executive refuses to average them.
    let outcome = run(input(
        vec![
            link("support", "a", "b", Relation::Supports, Sign::Supports, EvidenceClass::Established, Some(0.5)),
            link("inhibit", "a", "b", Relation::Inhibits, Sign::Opposes, EvidenceClass::Established, Some(0.5)),
        ],
        question(Goal::TraceCause, Some("a"), "b"),
    ));
    assert!(outcome.conflict, "the conflict is reported, never hidden");
    assert_eq!(outcome.status, Status::Unknown, "conflict fails closed to unknown");
    assert_eq!(outcome.direction, None);
    assert_eq!(outcome.rung, Rung::Creative, "unknown reasoning hands over to creativity");
}

#[test]
fn bridges_carry_provenance_and_evidence_class_only() {
    let mut first = link("shared-a", "a", "b", Relation::Supports, Sign::Supports, EvidenceClass::Established, Some(0.5));
    first.world = "w1".into();
    let mut second_link = link("shared-b", "a", "b", Relation::Causes, Sign::Supports, EvidenceClass::Established, Some(0.5));
    second_link.id = "shared-b".into();
    second_link.source_ids = vec!["shared-source".into()];
    first.source_ids = vec!["shared-source".into()];
    let outcome = run(input(vec![first, second_link], question(Goal::TraceCause, Some("a"), "b")));
    assert!(!outcome.bridges.is_empty(), "lanes citing the same source are bridged");
    let bridge = &outcome.bridges[0];
    assert!(
        bridge.source_ids.contains(&"shared-source".to_string()),
        "the bridge carries provenance: {bridge:?}"
    );
    assert_eq!(bridge.evidence_class, EvidenceClass::Established);
    // `Bridge` has no status, direction or weight field to carry — the doctrine's
    // "do not merge their stages, controls, evidence classes, or scores" holds by
    // construction, not by discipline.
}

#[test]
fn several_labelled_speculative_paths_answer_when_reasoning_cannot() {
    // One world, one direct path: induction stays unknown, so the creative lane
    // answers with the contracted shape.
    let outcome = run(input(
        vec![link("p", "a", "b", Relation::Causes, Sign::Supports, EvidenceClass::Provisional, None)],
        question(Goal::Generalize, Some("a"), "b"),
    ));
    assert_eq!(outcome.status, Status::Unknown, "no reasoning lane could stand on this");
    assert_eq!(outcome.rung, Rung::Creative);
    assert!(!outcome.creative.is_empty(), "the creative lane always produces");
    assert!(outcome.creative.len() <= 5, "the contract caps paths at five");
    for path in &outcome.creative {
        assert_eq!(path.status, "speculative_candidate_pending_prototype");
        assert_eq!(path.probability_kind, "uncalibrated_probability_of_feasibility_if_prototyped");
        assert!((0.05..=0.78).contains(&path.probability_score), "score {}", path.probability_score);
        let (low, high) = path.uncertainty_interval;
        assert!(low <= path.probability_score && path.probability_score <= high, "the band must contain the score");
        assert!(low >= 0.0 && high <= 1.0, "the band stays a probability");
        assert!(path.label.starts_with("Creative Path"), "paths are labelled: {}", path.label);
        assert!(!path.factual_claim_authority, "a path is never a factual claim");
        assert!(path.proposal_only && path.choice_records_intent_only);
    }
    assert!(!outcome.winner_declared, "no winner is declared");
    assert!(!outcome.factual_claim_authority);
}

#[test]
fn an_empty_admission_still_answers() {
    let outcome = run(input(vec![], question(Goal::Generalize, Some("a"), "b")));
    assert_eq!(outcome.rung, Rung::SpeculativeFallback, "the last rung catches it");
    assert_eq!(outcome.creative.len(), 1, "never empty, never several invented ones");
    let path = &outcome.creative[0];
    assert_eq!(path.label, "Speculative fallback");
    assert_eq!(path.probability_kind, "uncalibrated_probability_of_feasibility_if_prototyped");
    assert!(!path.factual_claim_authority && path.proposal_only);
    assert!(!outcome.factual_claim_authority);
    assert!(!outcome.winner_declared);
}

#[test]
fn a_lane_that_would_refuse_its_input_is_skipped_not_fatal() {
    // The counterfactual lane refuses a link that is not in the admission. The
    // engine raises; the executive must still answer, with that lane recorded as
    // skipped and the reason kept word for word.
    let mut counterfactual = question(Goal::AlterPremise, Some("a"), "b");
    counterfactual.removed_link = Some("absent-from-admission".into());
    let outcome = run(input(
        vec![link("p", "a", "b", Relation::Causes, Sign::Supports, EvidenceClass::Established, Some(0.6))],
        counterfactual,
    ));
    let skipped = outcome
        .positions
        .iter()
        .find(|position| position.lane == Lane::Counterfactual)
        .expect("the counterfactual lane is reported");
    assert_eq!(skipped.status, Status::Unknown);
    assert!(
        skipped.reason.contains("not in the admission"),
        "the refusal is the recorded reason: {}",
        skipped.reason
    );
    assert!(outcome.rung != Rung::Slow || outcome.status != Status::Unknown, "an answer still exists");
    assert!(!outcome.factual_claim_authority);
    assert!(!outcome.winner_declared);
}

#[test]
fn no_output_path_ever_claims_factual_authority() {
    for input in [
        input(vec![established("a", "b", Relation::Causes, Sign::Supports)], question(Goal::Prove, Some("a"), "b")),
        input(vec![link("p", "a", "b", Relation::Causes, Sign::Supports, EvidenceClass::Provisional, None)], question(Goal::Generalize, Some("a"), "b")),
        input(vec![], question(Goal::Explain, None, "b")),
    ] {
        let outcome = run(input);
        assert!(!outcome.factual_claim_authority);
        assert!(!outcome.winner_declared);
        assert!(outcome.creative.iter().all(|path| !path.factual_claim_authority));
    }
}
