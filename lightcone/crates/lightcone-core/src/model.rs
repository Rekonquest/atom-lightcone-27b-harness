use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldManifest {
    pub schema_version: u32,
    pub world_id: String,
    pub version: String,
    pub created_at: String,
    pub adapter_independent: bool,
    pub ontology_hash: String,
    pub domains: Vec<String>,
    pub lanes: Vec<String>,
    pub node_count: usize,
    pub edge_count: usize,
    pub coordinate_dimensions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<ShadowManifest>,
    pub files: BTreeMap<String, String>,
    pub training_receipt: String,
    pub validation_receipts: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShadowManifest {
    pub dimensions: usize,
    pub projection: String,
    pub coordinates: String,
    pub observers: String,
    pub observer_only: bool,
    pub used_for_admission: bool,
    pub all_spatial: bool,
    pub has_time_axis: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicClass {
    Formal,
    Established,
    Provisional,
    Hypothesis,
    Contested,
    Interpretive,
    Creative,
    SyntheticControl,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldNode {
    pub schema_version: u32,
    pub node_id: String,
    pub domain_id: String,
    pub subject_id: String,
    pub lanes: Vec<String>,
    pub label: String,
    pub statement: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub epistemic_class: EpistemicClass,
    #[serde(default)]
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub as_of: Option<String>,
    pub version_boundary: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    Causes,
    Enables,
    Supports,
    Grounds,
    Clarifies,
    Qualifies,
    Inhibits,
    Prevents,
    Contradicts,
    Undercuts,
    Counterexample,
    Prerequisite,
    PartOf,
    TransformsTo,
    AnalogousTo,
    Predicts,
    Measures,
    Implements,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeDirection {
    Forward,
    Bidirectional,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorldEdge {
    pub schema_version: u32,
    pub edge_id: String,
    pub source_node: String,
    pub target_node: String,
    pub kind: RelationKind,
    pub polarity: i8,
    pub direction: EdgeDirection,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub exceptions: Vec<String>,
    #[serde(default)]
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRegistry {
    pub schema_version: u32,
    pub sources: Vec<SourceRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub source_id: String,
    pub title: String,
    pub locator: String,
    pub publisher: String,
    pub version: Option<String>,
    pub retrieved_at: String,
    pub rights: String,
    pub trust_role: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lexicon {
    pub schema_version: u32,
    pub entries: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Coordinates {
    pub schema_version: u32,
    pub dimensions: usize,
    pub vectors: Vec<NodeVector>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NodeVector {
    pub node_id: String,
    pub values: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowCellQuantization {
    pub method: String,
    pub cell_width: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowCertificate {
    pub non_expansive: bool,
    pub orthonormal_max_error: f64,
    pub spectral_norm: f64,
    pub maximum_distance_expansion: f64,
    pub distance_expansion_violations: usize,
    pub tolerance: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowProjection {
    pub schema_version: u32,
    pub projection_id: String,
    pub world_id: String,
    pub observer_only: bool,
    pub used_for_admission: bool,
    pub all_spatial: bool,
    pub has_time_axis: bool,
    pub source_dimensions: usize,
    pub target_dimensions: usize,
    pub axis_names: Vec<String>,
    pub metric: String,
    pub method: String,
    pub fit_split: String,
    pub fit_node_count: usize,
    pub fit_node_ids_sha256: String,
    pub source_coordinates_sha256: String,
    pub basis_sha256: String,
    pub center: Vec<f64>,
    pub basis: Vec<Vec<f64>>,
    pub train_rank: usize,
    pub explained_variance_ratio: Vec<f64>,
    pub cell_quantization: ShadowCellQuantization,
    pub reach_radius: f64,
    pub random_basis_placebo_seed: u64,
    pub axis_shuffle_placebo_seed: u64,
    pub axis_shuffle_permutation: Vec<usize>,
    pub certificate: ShadowCertificate,
    pub deterministic_replay: bool,
    pub deterministic_replay_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowCoordinates {
    pub schema_version: u32,
    pub projection_id: String,
    pub dimensions: usize,
    pub axis_names: Vec<String>,
    pub vectors: Vec<NodeVector>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShadowViewReceipt {
    pub schema_version: u32,
    pub world_id: String,
    pub world_version: String,
    pub manifest_hash: String,
    pub projection_id: String,
    pub projection_artifact_hash: String,
    pub node_id: String,
    pub all_spatial: bool,
    pub has_time_axis: bool,
    pub observer_only: bool,
    pub used_for_admission: bool,
    pub holds_verdict: bool,
    pub source_dimensions: usize,
    pub shadow_dimensions: usize,
    pub observer_dimensions: usize,
    pub metric: String,
    pub dropped_axes: Vec<String>,
    pub kept_axes: Vec<String>,
    pub shadow_values: Vec<f32>,
    pub observer_values: Vec<f32>,
    pub quantized_cell: Vec<i64>,
}
