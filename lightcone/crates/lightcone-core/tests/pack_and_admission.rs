use lightcone_core::{AdmissionPolicy, ConeDirection, PackError, WorldPack};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NONCE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn create() -> Self {
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("new-lightcone-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).expect("create fixture root");

        let sources = json!({
            "schema_version": 1,
            "sources": [{
                "source_id": "source.physics",
                "title": "Fixture mechanics source",
                "locator": "fixture://physics",
                "publisher": "fixture",
                "version": "1",
                "retrieved_at": "2026-09-03",
                "rights": "green",
                "trust_role": "test"
            }]
        });
        write_json(root.join("sources.json"), &sources);

        let nodes = [
            json!({
                "schema_version": 1, "node_id": "mechanics.fall", "domain_id": "physics",
                "subject_id": "physics.classical-mechanics", "lanes": ["reasoning.causal"],
                "label": "falling object", "statement": "A released object accelerates under the modeled gravitational field.",
                "aliases": ["falling body"], "epistemic_class": "established", "source_ids": ["source.physics"],
                "scope": "fixture mechanics", "limitations": [], "as_of": null, "version_boundary": "fixture-v1"
            }),
            json!({
                "schema_version": 1, "node_id": "mechanics.acceleration", "domain_id": "physics",
                "subject_id": "physics.classical-mechanics", "lanes": ["reasoning.causal"],
                "label": "acceleration", "statement": "Net force changes velocity according to the model.",
                "aliases": [], "epistemic_class": "formal", "source_ids": ["source.physics"],
                "scope": "fixture mechanics", "limitations": [], "as_of": null, "version_boundary": "fixture-v1"
            }),
            json!({
                "schema_version": 1, "node_id": "mechanics.impact", "domain_id": "physics",
                "subject_id": "physics.classical-mechanics", "lanes": ["reasoning.causal"],
                "label": "impact", "statement": "Contact changes the motion boundary conditions.",
                "aliases": [], "epistemic_class": "established", "source_ids": ["source.physics"],
                "scope": "fixture mechanics", "limitations": [], "as_of": null, "version_boundary": "fixture-v1"
            }),
            json!({
                "schema_version": 1, "node_id": "optics.refraction", "domain_id": "physics",
                "subject_id": "physics.optics", "lanes": ["reasoning.causal"],
                "label": "refraction", "statement": "A wave changes direction across media under the fixture law.",
                "aliases": [], "epistemic_class": "established", "source_ids": ["source.physics"],
                "scope": "fixture optics", "limitations": [], "as_of": null, "version_boundary": "fixture-v1"
            }),
        ];
        write_jsonl(root.join("nodes.jsonl"), &nodes);

        let edges = [
            json!({
                "schema_version": 1, "edge_id": "edge.fall-acceleration", "source_node": "mechanics.fall",
                "target_node": "mechanics.acceleration", "kind": "causes", "polarity": 1,
                "direction": "forward", "conditions": ["object is released"], "exceptions": [],
                "source_ids": ["source.physics"]
            }),
            json!({
                "schema_version": 1, "edge_id": "edge.acceleration-impact", "source_node": "mechanics.acceleration",
                "target_node": "mechanics.impact", "kind": "enables", "polarity": 1,
                "direction": "forward", "conditions": ["a surface intersects the trajectory"], "exceptions": [],
                "source_ids": ["source.physics"]
            }),
        ];
        write_jsonl(root.join("edges.jsonl"), &edges);
        write_json(
            root.join("lexicon.json"),
            &json!({
                "schema_version": 1,
                "entries": {
                    "acceleration": ["mechanics.acceleration"],
                    "falling body": ["mechanics.fall"],
                    "falling object": ["mechanics.fall"],
                    "impact": ["mechanics.impact"],
                    "refraction": ["optics.refraction"]
                }
            }),
        );
        write_json(
            root.join("receipts-training.json"),
            &json!({"fixture": true}),
        );
        write_json(
            root.join("receipts-validation.json"),
            &json!({"fixture": true, "passed": true}),
        );

        let names = [
            "nodes.jsonl",
            "edges.jsonl",
            "sources.json",
            "lexicon.json",
            "receipts-training.json",
            "receipts-validation.json",
        ];
        let files = names
            .into_iter()
            .map(|name| (name.to_string(), hash_file(&root.join(name))))
            .collect::<BTreeMap<_, _>>();
        write_json(
            root.join("manifest.json"),
            &json!({
                "schema_version": 1,
                "world_id": "fixture-mechanics",
                "version": "1.0.0",
                "created_at": "2026-09-03T00:00:00Z",
                "adapter_independent": true,
                "ontology_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "domains": ["physics"],
                "lanes": ["reasoning.causal"],
                "node_count": 4,
                "edge_count": 2,
                "coordinate_dimensions": null,
                "files": files,
                "training_receipt": "receipts-training.json",
                "validation_receipts": ["receipts-validation.json"],
                "limitations": ["test fixture"]
            }),
        );
        Self { root }
    }

    fn create_with_shadow() -> Self {
        let fixture = Self::create();
        let vectors = [
            ("mechanics.fall", [0.0, 0.0, 0.0, 0.0]),
            ("mechanics.acceleration", [1.0, 0.0, 0.0, 0.0]),
            ("mechanics.impact", [2.0, 0.0, 0.0, 0.0]),
            ("optics.refraction", [0.0, 3.0, 0.0, 0.0]),
        ];
        let encoded_vectors = vectors
            .iter()
            .map(|(node_id, values)| json!({"node_id": node_id, "values": values}))
            .collect::<Vec<_>>();
        write_json(
            fixture.root.join("coordinates.json"),
            &json!({
                "schema_version": 1,
                "dimensions": 4,
                "vectors": encoded_vectors
            }),
        );
        write_json(
            fixture.root.join("shadow-coordinates.json"),
            &json!({
                "schema_version": 1,
                "projection_id": "fixture-shadow4-v1",
                "dimensions": 4,
                "axis_names": ["x", "y", "z", "w"],
                "vectors": vectors.iter().map(|(node_id, values)| {
                    json!({"node_id": node_id, "values": values})
                }).collect::<Vec<_>>()
            }),
        );
        write_json(
            fixture.root.join("shadow-projection.json"),
            &json!({
                "schema_version": 1,
                "projection_id": "fixture-shadow4-v1",
                "world_id": "fixture-mechanics",
                "observer_only": true,
                "used_for_admission": false,
                "all_spatial": true,
                "has_time_axis": false,
                "source_dimensions": 4,
                "target_dimensions": 4,
                "axis_names": ["x", "y", "z", "w"],
                "metric": "euclidean_l2",
                "method": "train_split_centered_pca_with_orthonormal_rows",
                "fit_split": "train",
                "fit_node_count": 4,
                "fit_node_ids_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "source_coordinates_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "basis_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "center": [0.0, 0.0, 0.0, 0.0],
                "basis": [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0]
                ],
                "train_rank": 2,
                "explained_variance_ratio": [0.7, 0.3, 0.0, 0.0],
                "cell_quantization": {"method": "nearest_integer_cell", "cell_width": 0.25},
                "reach_radius": 1.25,
                "random_basis_placebo_seed": 17,
                "axis_shuffle_placebo_seed": 19,
                "axis_shuffle_permutation": [0, 1, 2, 3],
                "certificate": {
                    "non_expansive": true,
                    "orthonormal_max_error": 0.0,
                    "spectral_norm": 1.0,
                    "maximum_distance_expansion": 0.0,
                    "distance_expansion_violations": 0,
                    "tolerance": 0.00001
                },
                "deterministic_replay": true,
                "deterministic_replay_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
            }),
        );
        write_json(
            fixture.root.join("shadow-observers.json"),
            &json!({
                "schema_version": 1,
                "projection_id": "fixture-shadow4-v1",
                "observer_only": true,
                "composition": {
                    "three_dimensional_exact": true,
                    "two_dimensional_exact": true
                },
                "three_dimensional": [],
                "two_dimensional": []
            }),
        );
        let mut manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(fixture.root.join("manifest.json")).expect("read manifest"),
        )
        .expect("parse manifest");
        manifest["coordinate_dimensions"] = json!(4);
        manifest["shadow"] = json!({
            "dimensions": 4,
            "projection": "shadow-projection.json",
            "coordinates": "shadow-coordinates.json",
            "observers": "shadow-observers.json",
            "observer_only": true,
            "used_for_admission": false,
            "all_spatial": true,
            "has_time_axis": false
        });
        for name in [
            "coordinates.json",
            "shadow-projection.json",
            "shadow-coordinates.json",
            "shadow-observers.json",
        ] {
            manifest["files"][name] = json!(hash_file(&fixture.root.join(name)));
        }
        write_json(fixture.root.join("manifest.json"), &manifest);
        fixture
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn verifies_every_hash_before_loading() {
    let fixture = Fixture::create();
    fs::write(fixture.root.join("lexicon.json"), "{}\n").expect("tamper fixture");
    let error = WorldPack::load(&fixture.root).expect_err("tampering must fail");
    assert!(matches!(error, PackError::HashMismatch { .. }));
}

#[test]
fn admits_exact_future_cone_without_unrelated_nodes() {
    let fixture = Fixture::create();
    let pack = WorldPack::load(&fixture.root).expect("valid pack");
    let policy = AdmissionPolicy {
        policy_id: "future-two".into(),
        max_hops: 2,
        max_nodes: 16,
        direction: ConeDirection::CausalFuture,
        allowed_relations: Default::default(),
    };
    let admission = pack
        .admit("Explain a falling object", policy)
        .expect("admission");
    assert_eq!(
        admission.receipt.admitted_nodes,
        [
            "mechanics.acceleration",
            "mechanics.fall",
            "mechanics.impact"
        ]
    );
    assert!(
        !admission
            .receipt
            .admitted_nodes
            .contains(&"optics.refraction".to_string())
    );
    assert_eq!(admission.receipt.traversed_edges.len(), 2);
    assert!(admission.receipt.passive);
    assert!(!admission.receipt.holds_verdict);
}

#[test]
fn direction_and_hop_limit_are_exact() {
    let fixture = Fixture::create();
    let pack = WorldPack::load(&fixture.root).expect("valid pack");
    let admission = pack
        .admit(
            "impact",
            AdmissionPolicy {
                policy_id: "past-one".into(),
                max_hops: 1,
                max_nodes: 16,
                direction: ConeDirection::CausalPast,
                allowed_relations: Default::default(),
            },
        )
        .expect("admission");
    assert_eq!(
        admission.receipt.admitted_nodes,
        ["mechanics.acceleration", "mechanics.impact"]
    );
}

#[test]
fn unknown_query_is_empty_and_deterministic() {
    let fixture = Fixture::create();
    let pack = WorldPack::load(&fixture.root).expect("valid pack");
    let first = pack
        .admit("unrepresented subject", AdmissionPolicy::default())
        .expect("first");
    let second = pack
        .admit("unrepresented subject", AdmissionPolicy::default())
        .expect("second");
    assert!(first.nodes.is_empty());
    assert!(first.edges.is_empty());
    assert_eq!(first.receipt, second.receipt);
}

#[test]
fn bounded_cone_reports_truncation() {
    let fixture = Fixture::create();
    let pack = WorldPack::load(&fixture.root).expect("valid pack");
    let admission = pack
        .admit(
            "falling object",
            AdmissionPolicy {
                policy_id: "bounded".into(),
                max_hops: 2,
                max_nodes: 2,
                direction: ConeDirection::CausalFuture,
                allowed_relations: Default::default(),
            },
        )
        .expect("admission");
    assert!(admission.receipt.truncated);
    assert_eq!(admission.receipt.excluded_frontier_count, 1);
    assert_eq!(admission.receipt.admitted_nodes.len(), 2);
}

#[test]
fn four_dimensional_shadow_is_verified_and_observer_only() {
    let fixture = Fixture::create_with_shadow();
    let pack = WorldPack::load(&fixture.root).expect("valid shadow pack");
    let receipt = pack
        .shadow_view("mechanics.impact", &["y".to_string(), "w".to_string()])
        .expect("shadow view");
    assert_eq!(receipt.shadow_values, vec![2.0, 0.0, 0.0, 0.0]);
    assert_eq!(receipt.kept_axes, vec!["x", "z"]);
    assert_eq!(receipt.observer_values, vec![2.0, 0.0]);
    assert_eq!(receipt.observer_dimensions, 2);
    assert!(receipt.all_spatial);
    assert!(!receipt.has_time_axis);
    assert!(receipt.observer_only);
    assert!(!receipt.used_for_admission);
    assert!(!receipt.holds_verdict);

    let admission = pack
        .admit("falling object", AdmissionPolicy::default())
        .expect("graph admission remains available");
    assert!(admission.receipt.passive);
    assert_eq!(admission.receipt.seed_nodes, ["mechanics.fall"]);
}

#[test]
fn shadow_coordinates_must_match_registered_projection() {
    let fixture = Fixture::create_with_shadow();
    let path = fixture.root.join("shadow-coordinates.json");
    let mut shadow: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read shadow")).expect("parse shadow");
    shadow["vectors"][0]["values"][0] = json!(99.0);
    write_json(path.clone(), &shadow);
    let manifest_path = fixture.root.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    manifest["files"]["shadow-coordinates.json"] = json!(hash_file(&path));
    write_json(manifest_path, &manifest);
    let error = WorldPack::load(&fixture.root).expect_err("invalid derived shadow must fail");
    assert!(matches!(error, PackError::Invalid(_)));
}

fn write_json(path: PathBuf, value: &serde_json::Value) {
    fs::write(path, serde_json::to_vec_pretty(value).expect("serialize")).expect("write JSON");
}

fn write_jsonl(path: PathBuf, values: &[serde_json::Value]) {
    let mut text = values
        .iter()
        .map(|value| serde_json::to_string(value).expect("serialize row"))
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    fs::write(path, text).expect("write JSONL");
}

fn hash_file(path: &Path) -> String {
    format!(
        "{:x}",
        Sha256::digest(fs::read(path).expect("read for hash"))
    )
}
