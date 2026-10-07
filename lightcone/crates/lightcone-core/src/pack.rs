use crate::model::{
    Coordinates, EdgeDirection, Lexicon, ShadowCoordinates, ShadowManifest, ShadowProjection,
    ShadowViewReceipt, SourceRegistry, WorldEdge, WorldManifest, WorldNode,
};
use crate::normalize::normalize_text;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("invalid world pack: {0}")]
    Invalid(String),
    #[error("hash mismatch for {path}: expected {expected}, observed {observed}")]
    HashMismatch {
        path: String,
        expected: String,
        observed: String,
    },
}

#[derive(Clone, Debug)]
pub struct WorldPack {
    pub root: PathBuf,
    pub manifest: WorldManifest,
    pub manifest_hash: String,
    pub nodes: BTreeMap<String, WorldNode>,
    pub edges: BTreeMap<String, WorldEdge>,
    pub sources: SourceRegistry,
    pub lexicon: BTreeMap<String, Vec<String>>,
    pub coordinates: Option<Coordinates>,
    pub shadow_projection: Option<ShadowProjection>,
    pub shadow_coordinates: Option<ShadowCoordinates>,
    pub(crate) outgoing: BTreeMap<String, Vec<String>>,
    pub(crate) incoming: BTreeMap<String, Vec<String>>,
}

impl WorldPack {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, PackError> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join("manifest.json");
        let manifest_bytes = read_bytes(&manifest_path)?;
        let manifest_hash = sha256(&manifest_bytes);
        let manifest: WorldManifest = parse_json(&manifest_path, &manifest_bytes)?;
        validate_manifest(&manifest)?;

        for (relative, expected) in &manifest.files {
            let path = safe_join(&root, relative)?;
            let observed = sha256(&read_bytes(&path)?);
            if observed != *expected {
                return Err(PackError::HashMismatch {
                    path: relative.clone(),
                    expected: expected.clone(),
                    observed,
                });
            }
        }
        for required in ["nodes.jsonl", "edges.jsonl", "sources.json", "lexicon.json"] {
            if !manifest.files.contains_key(required) {
                return Err(PackError::Invalid(format!(
                    "manifest does not hash required file {required}"
                )));
            }
        }
        if !manifest.files.contains_key(&manifest.training_receipt) {
            return Err(PackError::Invalid(
                "training receipt is not hash-bound by the manifest".into(),
            ));
        }
        for receipt in &manifest.validation_receipts {
            if !manifest.files.contains_key(receipt) {
                return Err(PackError::Invalid(format!(
                    "validation receipt {receipt} is not hash-bound by the manifest"
                )));
            }
        }

        let nodes = parse_jsonl::<WorldNode, _>(&root.join("nodes.jsonl"), "node_id", |node| {
            &node.node_id
        })?;
        let edges = parse_jsonl::<WorldEdge, _>(&root.join("edges.jsonl"), "edge_id", |edge| {
            &edge.edge_id
        })?;
        let sources: SourceRegistry = parse_file(&root.join("sources.json"))?;
        let lexicon: Lexicon = parse_file(&root.join("lexicon.json"))?;
        let coordinates = if manifest.files.contains_key("coordinates.json") {
            Some(parse_file::<Coordinates>(&root.join("coordinates.json"))?)
        } else {
            None
        };
        let (shadow_projection, shadow_coordinates) = if let Some(shadow) = &manifest.shadow {
            (
                Some(parse_file::<ShadowProjection>(&safe_join(
                    &root,
                    &shadow.projection,
                )?)?),
                Some(parse_file::<ShadowCoordinates>(&safe_join(
                    &root,
                    &shadow.coordinates,
                )?)?),
            )
        } else {
            (None, None)
        };
        validate_contents(
            &manifest,
            &nodes,
            &edges,
            &sources,
            &lexicon,
            coordinates.as_ref(),
            (shadow_projection.as_ref(), shadow_coordinates.as_ref()),
        )?;

        let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut incoming: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for edge in edges.values() {
            outgoing
                .entry(edge.source_node.clone())
                .or_default()
                .push(edge.edge_id.clone());
            incoming
                .entry(edge.target_node.clone())
                .or_default()
                .push(edge.edge_id.clone());
            if edge.direction == EdgeDirection::Bidirectional {
                outgoing
                    .entry(edge.target_node.clone())
                    .or_default()
                    .push(edge.edge_id.clone());
                incoming
                    .entry(edge.source_node.clone())
                    .or_default()
                    .push(edge.edge_id.clone());
            }
        }
        for ids in outgoing.values_mut().chain(incoming.values_mut()) {
            ids.sort();
            ids.dedup();
        }

        Ok(Self {
            root,
            manifest,
            manifest_hash,
            nodes,
            edges,
            sources,
            lexicon: lexicon.entries,
            coordinates,
            shadow_projection,
            shadow_coordinates,
            outgoing,
            incoming,
        })
    }

    pub fn shadow_view(
        &self,
        node_id: &str,
        dropped_axes: &[String],
    ) -> Result<ShadowViewReceipt, PackError> {
        let shadow_manifest = self.manifest.shadow.as_ref().ok_or_else(|| {
            PackError::Invalid("world pack has no four-dimensional shadow".into())
        })?;
        let projection = self.shadow_projection.as_ref().ok_or_else(|| {
            PackError::Invalid("world pack did not load its shadow projection".into())
        })?;
        let coordinates = self.shadow_coordinates.as_ref().ok_or_else(|| {
            PackError::Invalid("world pack did not load its shadow coordinates".into())
        })?;
        if !self.nodes.contains_key(node_id) {
            return Err(PackError::Invalid(format!("unknown shadow node {node_id}")));
        }
        let mut requested = BTreeSet::new();
        for axis in dropped_axes {
            if !projection.axis_names.contains(axis) {
                return Err(PackError::Invalid(format!(
                    "unknown shadow axis {axis:?}; expected x, y, z, or w"
                )));
            }
            if !requested.insert(axis.clone()) {
                return Err(PackError::Invalid(format!(
                    "shadow axis {axis:?} was dropped more than once"
                )));
            }
        }
        if requested.len() > 2 {
            return Err(PackError::Invalid(
                "observer views retain at least two of the four spatial axes".into(),
            ));
        }
        let vector = coordinates
            .vectors
            .iter()
            .find(|vector| vector.node_id == node_id)
            .ok_or_else(|| PackError::Invalid(format!("shadow has no vector for {node_id}")))?;
        let kept_indices = projection
            .axis_names
            .iter()
            .enumerate()
            .filter(|(_, axis)| !requested.contains(*axis))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let kept_axes = kept_indices
            .iter()
            .map(|index| projection.axis_names[*index].clone())
            .collect::<Vec<_>>();
        let observer_values = kept_indices
            .iter()
            .map(|index| vector.values[*index])
            .collect::<Vec<_>>();
        let width = projection.cell_quantization.cell_width;
        let quantized_cell = vector
            .values
            .iter()
            .map(|value| (f64::from(*value) / width + 0.5).floor() as i64)
            .collect();
        let projection_artifact_hash = self
            .manifest
            .files
            .get(&shadow_manifest.projection)
            .cloned()
            .ok_or_else(|| PackError::Invalid("shadow projection is not hash-bound".into()))?;
        Ok(ShadowViewReceipt {
            schema_version: 1,
            world_id: self.manifest.world_id.clone(),
            world_version: self.manifest.version.clone(),
            manifest_hash: self.manifest_hash.clone(),
            projection_id: projection.projection_id.clone(),
            projection_artifact_hash,
            node_id: node_id.to_string(),
            all_spatial: true,
            has_time_axis: false,
            observer_only: true,
            used_for_admission: false,
            holds_verdict: false,
            source_dimensions: projection.source_dimensions,
            shadow_dimensions: projection.target_dimensions,
            observer_dimensions: observer_values.len(),
            metric: projection.metric.clone(),
            dropped_axes: projection
                .axis_names
                .iter()
                .filter(|axis| requested.contains(*axis))
                .cloned()
                .collect(),
            kept_axes,
            shadow_values: vector.values.clone(),
            observer_values,
            quantized_cell,
        })
    }
}

fn validate_manifest(manifest: &WorldManifest) -> Result<(), PackError> {
    if manifest.schema_version != 1 {
        return Err(PackError::Invalid(format!(
            "unsupported manifest schema {}",
            manifest.schema_version
        )));
    }
    if !manifest.adapter_independent {
        return Err(PackError::Invalid("world pack is model-specific".into()));
    }
    if manifest.world_id.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err(PackError::Invalid(
            "world_id and version must be non-empty".into(),
        ));
    }
    if manifest.domains.is_empty() || manifest.lanes.is_empty() {
        return Err(PackError::Invalid(
            "world pack must declare domains and lanes".into(),
        ));
    }
    if manifest.validation_receipts.is_empty() {
        return Err(PackError::Invalid(
            "world pack has no validation receipt".into(),
        ));
    }
    if let Some(shadow) = &manifest.shadow {
        if shadow.dimensions != 4
            || !shadow.observer_only
            || shadow.used_for_admission
            || !shadow.all_spatial
            || shadow.has_time_axis
        {
            return Err(PackError::Invalid(
                "manifest shadow is not an observer-only four-dimensional all-spatial view".into(),
            ));
        }
        for path in [&shadow.projection, &shadow.coordinates, &shadow.observers] {
            validate_relative(path)?;
            if !manifest.files.contains_key(path) {
                return Err(PackError::Invalid(format!(
                    "shadow artifact {path} is not hash-bound by the manifest"
                )));
            }
        }
        if !manifest.files.contains_key("coordinates.json") {
            return Err(PackError::Invalid(
                "shadow projection requires hash-bound full coordinates".into(),
            ));
        }
    }
    for (path, digest) in &manifest.files {
        validate_relative(path)?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(PackError::Invalid(format!("invalid SHA-256 for {path}")));
        }
    }
    Ok(())
}

fn validate_contents(
    manifest: &WorldManifest,
    nodes: &BTreeMap<String, WorldNode>,
    edges: &BTreeMap<String, WorldEdge>,
    sources: &SourceRegistry,
    lexicon: &Lexicon,
    coordinates: Option<&Coordinates>,
    shadow: (Option<&ShadowProjection>, Option<&ShadowCoordinates>),
) -> Result<(), PackError> {
    if nodes.len() != manifest.node_count || edges.len() != manifest.edge_count {
        return Err(PackError::Invalid(format!(
            "manifest counts differ: nodes {} != {}, edges {} != {}",
            manifest.node_count,
            nodes.len(),
            manifest.edge_count,
            edges.len()
        )));
    }
    if nodes.is_empty() {
        return Err(PackError::Invalid("world pack contains no nodes".into()));
    }
    let source_ids = sources
        .sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect::<BTreeSet<_>>();
    if source_ids.len() != sources.sources.len() {
        return Err(PackError::Invalid(
            "source registry contains duplicate IDs".into(),
        ));
    }
    for node in nodes.values() {
        if node.schema_version != 1 {
            return Err(PackError::Invalid(format!(
                "node {} has unsupported schema",
                node.node_id
            )));
        }
        if !manifest.domains.contains(&node.domain_id) {
            return Err(PackError::Invalid(format!(
                "node {} uses undeclared domain {}",
                node.node_id, node.domain_id
            )));
        }
        if node.lanes.is_empty() || node.lanes.iter().any(|lane| !manifest.lanes.contains(lane)) {
            return Err(PackError::Invalid(format!(
                "node {} uses an empty or undeclared lane set",
                node.node_id
            )));
        }
        if node
            .source_ids
            .iter()
            .any(|source| !source_ids.contains(source.as_str()))
        {
            return Err(PackError::Invalid(format!(
                "node {} references an unknown source",
                node.node_id
            )));
        }
        if node.epistemic_class != crate::model::EpistemicClass::SyntheticControl
            && node.source_ids.is_empty()
        {
            return Err(PackError::Invalid(format!(
                "non-synthetic node {} has no evidence",
                node.node_id
            )));
        }
    }
    for edge in edges.values() {
        if edge.schema_version != 1 || !(-1..=1).contains(&edge.polarity) {
            return Err(PackError::Invalid(format!(
                "edge {} has an invalid schema or polarity",
                edge.edge_id
            )));
        }
        if !nodes.contains_key(&edge.source_node) || !nodes.contains_key(&edge.target_node) {
            return Err(PackError::Invalid(format!(
                "edge {} references an unknown node",
                edge.edge_id
            )));
        }
        if edge.source_ids.is_empty()
            || edge
                .source_ids
                .iter()
                .any(|source| !source_ids.contains(source.as_str()))
        {
            return Err(PackError::Invalid(format!(
                "edge {} has missing or unknown evidence",
                edge.edge_id
            )));
        }
    }
    for (raw_term, node_ids) in &lexicon.entries {
        if normalize_text(raw_term) != *raw_term || raw_term.is_empty() {
            return Err(PackError::Invalid(format!(
                "lexicon term is not canonical: {raw_term:?}"
            )));
        }
        if node_ids.is_empty() || node_ids.iter().any(|node| !nodes.contains_key(node)) {
            return Err(PackError::Invalid(format!(
                "lexicon term {raw_term:?} has no valid targets"
            )));
        }
    }
    if let Some(coordinates) = coordinates {
        if coordinates.schema_version != 1
            || Some(coordinates.dimensions) != manifest.coordinate_dimensions
        {
            return Err(PackError::Invalid(
                "coordinate metadata differs from manifest".into(),
            ));
        }
        let vector_ids = coordinates
            .vectors
            .iter()
            .map(|vector| vector.node_id.as_str())
            .collect::<BTreeSet<_>>();
        if vector_ids.len() != nodes.len()
            || !nodes.keys().all(|node| vector_ids.contains(node.as_str()))
        {
            return Err(PackError::Invalid(
                "coordinates do not cover every node exactly once".into(),
            ));
        }
        for vector in &coordinates.vectors {
            if vector.values.len() != coordinates.dimensions
                || vector.values.iter().any(|value| !value.is_finite())
            {
                return Err(PackError::Invalid(format!(
                    "node {} has an invalid coordinate vector",
                    vector.node_id
                )));
            }
        }
    } else if manifest.coordinate_dimensions.is_some() {
        return Err(PackError::Invalid(
            "manifest declares coordinates but coordinates.json is absent".into(),
        ));
    }
    match (manifest.shadow.as_ref(), coordinates, shadow.0, shadow.1) {
        (Some(shadow_manifest), Some(full), Some(projection), Some(shadow)) => {
            validate_shadow(manifest, shadow_manifest, nodes, full, projection, shadow)?;
        }
        (None, _, None, None) => {}
        _ => {
            return Err(PackError::Invalid(
                "manifest and loaded shadow artifacts are inconsistent".into(),
            ));
        }
    }
    Ok(())
}

fn validate_shadow(
    manifest: &WorldManifest,
    shadow_manifest: &ShadowManifest,
    nodes: &BTreeMap<String, WorldNode>,
    full: &Coordinates,
    projection: &ShadowProjection,
    shadow: &ShadowCoordinates,
) -> Result<(), PackError> {
    let expected_axes = ["x", "y", "z", "w"];
    if projection.schema_version != 1
        || projection.world_id != manifest.world_id
        || projection.projection_id.trim().is_empty()
        || !projection.observer_only
        || projection.used_for_admission
        || !projection.all_spatial
        || projection.has_time_axis
        || projection.source_dimensions != full.dimensions
        || projection.target_dimensions != 4
        || projection
            .axis_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_axes
        || projection.metric != "euclidean_l2"
        || projection.method != "train_split_centered_pca_with_orthonormal_rows"
        || projection.fit_split != "train"
        || projection.fit_node_count < 2
        || projection.fit_node_count > nodes.len()
        || !projection.deterministic_replay
    {
        return Err(PackError::Invalid(
            "shadow projection metadata violates the four-dimensional observer contract".into(),
        ));
    }
    if shadow_manifest.dimensions != 4
        || shadow.schema_version != 1
        || shadow.projection_id != projection.projection_id
        || shadow.dimensions != 4
        || shadow
            .axis_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_axes
    {
        return Err(PackError::Invalid(
            "shadow coordinate metadata differs from its projection".into(),
        ));
    }
    for digest in [
        &projection.fit_node_ids_sha256,
        &projection.source_coordinates_sha256,
        &projection.basis_sha256,
        &projection.deterministic_replay_sha256,
    ] {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(PackError::Invalid(
                "shadow projection contains an invalid SHA-256".into(),
            ));
        }
    }
    if projection.center.len() != full.dimensions
        || projection.basis.len() != 4
        || projection
            .basis
            .iter()
            .any(|row| row.len() != full.dimensions)
        || projection.center.iter().any(|value| !value.is_finite())
        || projection
            .basis
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        || projection.explained_variance_ratio.len() != 4
        || projection
            .explained_variance_ratio
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        || !projection.cell_quantization.cell_width.is_finite()
        || projection.cell_quantization.cell_width <= 0.0
        || projection.cell_quantization.method != "nearest_integer_cell"
        || !projection.reach_radius.is_finite()
        || projection.reach_radius <= 0.0
    {
        return Err(PackError::Invalid(
            "shadow projection contains invalid numeric structure".into(),
        ));
    }
    let mut permutation = projection.axis_shuffle_permutation.clone();
    permutation.sort_unstable();
    if permutation != (0..full.dimensions).collect::<Vec<_>>() {
        return Err(PackError::Invalid(
            "shadow axis-shuffle placebo is not a dimension permutation".into(),
        ));
    }
    let certificate = &projection.certificate;
    if !certificate.non_expansive
        || certificate.distance_expansion_violations != 0
        || !certificate.tolerance.is_finite()
        || certificate.tolerance <= 0.0
        || !certificate.orthonormal_max_error.is_finite()
        || !certificate.spectral_norm.is_finite()
        || !certificate.maximum_distance_expansion.is_finite()
        || certificate.maximum_distance_expansion > certificate.tolerance
        || certificate.spectral_norm > 1.0 + certificate.tolerance
    {
        return Err(PackError::Invalid(
            "shadow non-expansion certificate is invalid".into(),
        ));
    }
    let tolerance = certificate.tolerance.max(0.000_02);
    for left in 0..4 {
        for right in 0..4 {
            let dot = projection.basis[left]
                .iter()
                .zip(&projection.basis[right])
                .map(|(a, b)| a * b)
                .sum::<f64>();
            let expected = if left == right { 1.0 } else { 0.0 };
            if (dot - expected).abs() > tolerance {
                return Err(PackError::Invalid(
                    "shadow projection rows are not orthonormal".into(),
                ));
            }
        }
    }

    let full_by_id = full
        .vectors
        .iter()
        .map(|vector| (vector.node_id.as_str(), vector))
        .collect::<BTreeMap<_, _>>();
    let shadow_by_id = shadow
        .vectors
        .iter()
        .map(|vector| (vector.node_id.as_str(), vector))
        .collect::<BTreeMap<_, _>>();
    if full.vectors.len() != nodes.len()
        || shadow.vectors.len() != nodes.len()
        || shadow_by_id.len() != nodes.len()
        || !nodes
            .keys()
            .all(|node| shadow_by_id.contains_key(node.as_str()))
    {
        return Err(PackError::Invalid(
            "shadow coordinates do not cover every node exactly once".into(),
        ));
    }
    for node_id in nodes.keys() {
        let source = full_by_id[node_id.as_str()];
        let observed = shadow_by_id[node_id.as_str()];
        if observed.values.len() != 4 || observed.values.iter().any(|value| !value.is_finite()) {
            return Err(PackError::Invalid(format!(
                "node {node_id} has an invalid shadow vector"
            )));
        }
        for axis in 0..4 {
            let expected = source
                .values
                .iter()
                .zip(&projection.center)
                .zip(&projection.basis[axis])
                .map(|((value, center), weight)| (f64::from(*value) - center) * weight)
                .sum::<f64>();
            if (expected - f64::from(observed.values[axis])).abs() > tolerance {
                return Err(PackError::Invalid(format!(
                    "node {node_id} shadow vector does not match the registered projection"
                )));
            }
        }
    }
    let ordered = nodes
        .keys()
        .map(|node_id| {
            (
                full_by_id[node_id.as_str()].values.as_slice(),
                shadow_by_id[node_id.as_str()].values.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    for left in 0..ordered.len() {
        for right in left + 1..ordered.len() {
            let full_distance = ordered[left]
                .0
                .iter()
                .zip(ordered[right].0)
                .map(|(a, b)| {
                    let delta = f64::from(*a) - f64::from(*b);
                    delta * delta
                })
                .sum::<f64>()
                .sqrt();
            let shadow_distance = ordered[left]
                .1
                .iter()
                .zip(ordered[right].1)
                .map(|(a, b)| {
                    let delta = f64::from(*a) - f64::from(*b);
                    delta * delta
                })
                .sum::<f64>()
                .sqrt();
            if shadow_distance > full_distance + tolerance {
                return Err(PackError::Invalid(
                    "shadow coordinates violate non-expansion".into(),
                ));
            }
        }
    }
    Ok(())
}

fn parse_jsonl<T, F>(path: &Path, label: &str, id_of: F) -> Result<BTreeMap<String, T>, PackError>
where
    T: serde::de::DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let text = String::from_utf8(read_bytes(path)?)
        .map_err(|_| PackError::Invalid(format!("{} is not UTF-8", path.display())))?;
    let mut output = BTreeMap::new();
    for (line_index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: T = serde_json::from_str(line).map_err(|source| PackError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        let id = id_of(&value).to_string();
        if id.is_empty() || output.insert(id.clone(), value).is_some() {
            return Err(PackError::Invalid(format!(
                "duplicate or empty {label} {id:?} at line {}",
                line_index + 1
            )));
        }
    }
    Ok(output)
}

fn parse_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, PackError> {
    let bytes = read_bytes(path)?;
    parse_json(path, &bytes)
}

fn parse_json<T: serde::de::DeserializeOwned>(path: &Path, bytes: &[u8]) -> Result<T, PackError> {
    serde_json::from_slice(bytes).map_err(|source| PackError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, PackError> {
    fs::read(path).map_err(|source| PackError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_relative(relative: &str) -> Result<(), PackError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(PackError::Invalid(format!(
            "manifest path is not a safe relative path: {relative}"
        )));
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, PackError> {
    validate_relative(relative)?;
    Ok(root.join(relative))
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
