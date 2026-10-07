use atom_creativity::{all_operations, Brief, Frame, Tension};
use atom_orchestrator::{
    run_creative, Admission, Concept, CreativeInput, Engine, EvidenceClass, Goal, Input, Link,
    Question, Relation, Sign, Status,
};

fn input() -> CreativeInput {
    CreativeInput {
        reasoning: Input {
            admission: Admission {
                receipt_id: "synthetic-receipt".into(),
                links: vec![Link {
                    id: "source-path".into(),
                    world: "fixture".into(),
                    cause: "A".into(),
                    effect: "B".into(),
                    domain: "test".into(),
                    target_domain: "test".into(),
                    conditions: vec![],
                    exceptions: vec![],
                    scope: "fixture".into(),
                    target_scope: "fixture".into(),
                    source_ids: vec!["source-1".into()],
                    relation: Relation::Causes,
                    sign: Sign::Supports,
                    class: EvidenceClass::Hypothesis,
                    target_class: EvidenceClass::Hypothesis,
                    confidence: None,
                }],
            },
            question: Question {
                id: "fixture".into(),
                goal: Goal::TraceCause,
                cause: Some("A".into()),
                effect: "B".into(),
                allowed_domains: vec![],
                conditions: vec![],
                removed_link: None,
                analogy: None,
            },
        },
        brief: Brief {
            id: "fixture".into(),
            required: vec!["need".into()],
            preferred: vec![],
            forbidden_pairs: vec![],
            seed_ids: vec!["a".into()],
            frames: vec![Frame {
                id: "reframe".into(),
                aliases: vec![("need".into(), "bridge".into())],
                bonus_attributes: vec![],
            }],
            tensions: vec![Tension {
                left: "left".into(),
                right: "right".into(),
                bridge: "bridge".into(),
            }],
        },
        concepts: vec![
            Concept {
                id: "a".into(),
                space: "one".into(),
                attributes: vec!["left".into()],
                value: 0.7,
                memory: false,
            },
            Concept {
                id: "b".into(),
                space: "two".into(),
                attributes: vec!["right".into()],
                value: 0.8,
                memory: true,
            },
        ],
        config: all_operations().expect("seven operations"),
    }
}

#[test]
fn reasoning_paths_ground_creative_candidates_without_promoting_truth() {
    let report = run_creative(&Engine, input()).expect("composed execution");
    assert_eq!(report.reasoning.lanes[0].status, Status::Unknown);
    assert_eq!(report.creativity.grounding_source_ids, ["source-1"]);
    assert!(report
        .creativity
        .frontier
        .iter()
        .any(|candidate| candidate.attributes.contains(&"bridge".to_string())));
    assert!(!report.creativity.factual_claim_authority);
    assert!(!report.reasoning.factual_claim_authority);
}

#[test]
fn invalid_creative_input_fails_closed() {
    let mut invalid = input();
    invalid.brief.seed_ids = vec!["absent".into()];
    assert!(run_creative(&Engine, invalid).is_err());
}
