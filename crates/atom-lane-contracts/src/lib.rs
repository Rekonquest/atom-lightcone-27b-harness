//! Verification leaf for the seven reasoning-lane contracts.
//!
//! The lane contracts are tested from `tests/lanes.rs`, which needs
//! `atom-reasoning-engine` as a dev-dependency. Keeping them here rather than
//! inside the engine package is what lets the engine stay inside the build
//! contract's 500 physical lines (source + tests + manifest), which its own
//! lane implementations alone already fill.
//!
//! Nothing in the cognitive chain depends on this crate; it is verification
//! scaffolding, and it is deliberately empty of production code.
