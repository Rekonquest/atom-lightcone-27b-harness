//! Model-independent passive causal-cone admission.
//!
//! This crate verifies immutable world packs, resolves explicit lexicon seeds,
//! traverses exact typed graph edges under a bounded geometric policy, and
//! returns a deterministic receipt. It does not produce answers or verdicts.

mod admission;
mod model;
mod normalize;
mod pack;

pub use admission::{
    Admission, AdmissionError, AdmissionPolicy, AdmissionReceipt, ConeDirection, TraversedEdge,
};
pub use model::{
    Coordinates, EdgeDirection, EpistemicClass, Lexicon, NodeVector, RelationKind,
    ShadowCellQuantization, ShadowCertificate, ShadowCoordinates, ShadowManifest, ShadowProjection,
    ShadowViewReceipt, SourceRecord, SourceRegistry, WorldEdge, WorldManifest, WorldNode,
};
pub use normalize::{normalize_text, query_terms};
pub use pack::{PackError, WorldPack};
