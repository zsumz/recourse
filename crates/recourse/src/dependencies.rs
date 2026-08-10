//! Public dependencies re-exported at the major versions Recourse requires.
//!
//! Applications name these crates in their own diagnostic declarations. A
//! `schemars` or `serde` major mismatch surfaces as an unresolved trait bound
//! rather than a version error, so the exact compiled versions are re-exported
//! here. `recourse::http` is the protocol's own HTTP surface; the `http`
//! dependency is reachable through this module instead.

/// HTTP status, header, and method types named by public Recourse signatures.
pub use ::http;

/// Schema derivation required by every `PublicEvidence` implementation.
pub use ::schemars;

/// Serialization traits required by every `PublicEvidence` implementation.
pub use ::serde;
