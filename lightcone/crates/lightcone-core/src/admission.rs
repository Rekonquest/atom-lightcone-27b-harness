use crate::model::{RelationKind, WorldEdge, WorldNode};
use crate::normalize::query_terms;
use crate::pack::{WorldPack, sha256};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConeDirection {
    CausalPast,
    CausalFuture,
    Both,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdmissionPolicy {
    pub policy_id: String,
    pub max_hops: usize,
    pub max_nodes: usize,
    pub direction: ConeDirection,
    #[serde(default)]
    pub allowed_relations: BTreeSet<RelationKind>,
}

impl Default for AdmissionPolicy {
    fn default() -> Self {
        Self {
            policy_id: "causal-both-v1".into(),
            max_hops: 2,
            max_nodes: 256,
            direction: ConeDirection::Both,
            allowed_relations: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraversedEdge {
    pub edge_id: String,
    pub from: String,
    pub to: String,
    pub hop: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdmissionReceipt {
    pub schema_version: u32,
    pub world_id: String,
    pub world_version: String,
    pub manifest_hash: String,
    pub policy: AdmissionPolicy,
    pub query_hash: String,
    pub matched_terms: Vec<String>,
    pub seed_nodes: Vec<String>,
    pub admitted_nodes: Vec<String>,
    pub traversed_edges: Vec<TraversedEdge>,
    pub truncated: bool,
    pub excluded_frontier_count: usize,
    pub passive: bool,
    pub holds_verdict: bool,
    pub receipt_hash: String,
}

#[derive(Clone, Debug)]
pub struct Admission {
    pub nodes: Vec<WorldNode>,
    pub edges: Vec<WorldEdge>,
    pub receipt: AdmissionReceipt,
}

#[derive(Debug, Error)]
pub enum AdmissionError {
    #[error("invalid admission policy: {0}")]
    InvalidPolicy(String),
    #[error("could not encode admission receipt: {0}")]
    Receipt(#[from] serde_json::Error),
}

impl WorldPack {
    pub fn admit(&self, query: &str, policy: AdmissionPolicy) -> Result<Admission, AdmissionError> {
        validate_policy(&policy)?;
        let terms = query_terms(query);
        let mut matched_terms = Vec::new();
        let mut seed_set = BTreeSet::new();
        for term in terms {
            if let Some(nodes) = self.lexicon.get(&term) {
                matched_terms.push(term);
                seed_set.extend(nodes.iter().cloned());
            }
        }
        let seed_nodes = seed_set
            .iter()
            .take(policy.max_nodes)
            .cloned()
            .collect::<Vec<_>>();
        let mut admitted = seed_nodes.iter().cloned().collect::<BTreeSet<_>>();
        let mut queue = seed_nodes
            .iter()
            .cloned()
            .map(|node| (node, 0usize))
            .collect::<VecDeque<_>>();
        let mut traversed = BTreeMap::<String, TraversedEdge>::new();
        let mut excluded_frontier = BTreeSet::new();

        while let Some((node_id, hop)) = queue.pop_front() {
            if hop >= policy.max_hops {
                continue;
            }
            for (edge_id, neighbor) in self.neighbors(&node_id, &policy) {
                if admitted.contains(&neighbor) {
                    // The node is already in the cone, but the relation that
                    // reaches it is still real geometry and must reach the
                    // receipt. Dropping it here is what made "name both ends of
                    // the relation you are asking about" lose exactly that
                    // relation (reproduced 2026-09-24). The node is not
                    // re-queued, so the cone's shape is unchanged.
                    traversed.entry(edge_id.clone()).or_insert(TraversedEdge {
                        edge_id,
                        from: node_id.clone(),
                        to: neighbor.clone(),
                        hop: hop + 1,
                    });
                    continue;
                }
                if admitted.len() >= policy.max_nodes {
                    excluded_frontier.insert(neighbor);
                    continue;
                }
                admitted.insert(neighbor.clone());
                traversed.entry(edge_id.clone()).or_insert(TraversedEdge {
                    edge_id,
                    from: node_id.clone(),
                    to: neighbor.clone(),
                    hop: hop + 1,
                });
                queue.push_back((neighbor, hop + 1));
            }
        }

        let admitted_nodes = admitted.into_iter().collect::<Vec<_>>();
        let traversed_edges = traversed.into_values().collect::<Vec<_>>();
        let nodes = admitted_nodes
            .iter()
            .filter_map(|id| self.nodes.get(id).cloned())
            .collect::<Vec<_>>();
        let edge_ids = traversed_edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect::<BTreeSet<_>>();
        let edges = edge_ids
            .into_iter()
            .filter_map(|id| self.edges.get(id).cloned())
            .collect::<Vec<_>>();
        let query_hash = sha256(query.as_bytes());
        let mut receipt = AdmissionReceipt {
            schema_version: 1,
            world_id: self.manifest.world_id.clone(),
            world_version: self.manifest.version.clone(),
            manifest_hash: self.manifest_hash.clone(),
            policy,
            query_hash,
            matched_terms,
            seed_nodes,
            admitted_nodes,
            traversed_edges,
            truncated: !excluded_frontier.is_empty(),
            excluded_frontier_count: excluded_frontier.len(),
            passive: true,
            holds_verdict: false,
            receipt_hash: String::new(),
        };
        let canonical = serde_json::to_vec(&receipt)?;
        receipt.receipt_hash = sha256(&canonical);
        Ok(Admission {
            nodes,
            edges,
            receipt,
        })
    }

    fn neighbors(&self, node_id: &str, policy: &AdmissionPolicy) -> Vec<(String, String)> {
        let mut neighbors = Vec::new();
        if matches!(
            policy.direction,
            ConeDirection::CausalFuture | ConeDirection::Both
        ) {
            for edge_id in self.outgoing.get(node_id).into_iter().flatten() {
                if let Some(edge) = self.edges.get(edge_id) {
                    if !relation_allowed(edge, policy) {
                        continue;
                    }
                    let neighbor = if edge.source_node == node_id {
                        &edge.target_node
                    } else {
                        &edge.source_node
                    };
                    neighbors.push((edge_id.clone(), neighbor.clone()));
                }
            }
        }
        if matches!(
            policy.direction,
            ConeDirection::CausalPast | ConeDirection::Both
        ) {
            for edge_id in self.incoming.get(node_id).into_iter().flatten() {
                if let Some(edge) = self.edges.get(edge_id) {
                    if !relation_allowed(edge, policy) {
                        continue;
                    }
                    let neighbor = if edge.target_node == node_id {
                        &edge.source_node
                    } else {
                        &edge.target_node
                    };
                    neighbors.push((edge_id.clone(), neighbor.clone()));
                }
            }
        }
        neighbors.sort();
        neighbors.dedup();
        neighbors
    }
}

fn relation_allowed(edge: &WorldEdge, policy: &AdmissionPolicy) -> bool {
    policy.allowed_relations.is_empty() || policy.allowed_relations.contains(&edge.kind)
}

fn validate_policy(policy: &AdmissionPolicy) -> Result<(), AdmissionError> {
    if policy.policy_id.trim().is_empty() {
        return Err(AdmissionError::InvalidPolicy(
            "policy_id must not be empty".into(),
        ));
    }
    if policy.max_hops > 64 {
        return Err(AdmissionError::InvalidPolicy(
            "max_hops must be <= 64".into(),
        ));
    }
    if !(1..=1_000_000).contains(&policy.max_nodes) {
        return Err(AdmissionError::InvalidPolicy(
            "max_nodes must be within 1..=1,000,000".into(),
        ));
    }
    Ok(())
}
