use atom_creativity::{
    all_operations, explore, Brief, Concept, CreativeConfig, Frame, Operation, Tension,
};

#[test]
fn omitted_mechanisms_change_assembled_candidates() {
    let concepts = vec![
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
    ];
    let brief = Brief {
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
    };
    let base = all_operations().expect("config");
    let without = |operation| CreativeConfig {
        enabled: base
            .enabled
            .iter()
            .copied()
            .filter(|item| *item != operation)
            .collect(),
        ..base.clone()
    };
    let no_blend = explore(
        &brief,
        &concepts,
        &[],
        &without(Operation::ConceptualBlending),
    )
    .expect("ablation");
    assert!(no_blend.frontier.iter().all(|c| c.concept_ids.len() == 1));
    let no_tension = explore(
        &brief,
        &concepts,
        &[],
        &without(Operation::TensionTransformation),
    )
    .expect("ablation");
    assert!(no_tension
        .frontier
        .iter()
        .all(|c| !c.attributes.contains(&"bridge".into())));
    let no_memory = explore(
        &brief,
        &concepts,
        &[],
        &without(Operation::MemoryRecombination),
    )
    .expect("ablation");
    assert!(no_memory
        .frontier
        .iter()
        .all(|c| !c.concept_ids.contains(&"b".into())));
    let no_frame =
        explore(&brief, &concepts, &[], &without(Operation::ProblemFraming)).expect("ablation");
    assert!(no_frame.frontier.iter().all(|c| c.required_coverage == 0.0));
    assert!(no_frame.evaluation_only);
}
