//! Semantic metadata owned by one permanent diagnostic marker type.

use crate::catalog::{CatalogSpec, CodeNumber};

use super::PublicEvidence;

/// Declares stable identity, public evidence, and caller-facing guidance.
pub trait DiagnosticType: Send + Sync + 'static {
    /// Catalog namespace that owns this diagnostic identity.
    type Catalog: CatalogSpec;

    /// Reviewed caller-visible evidence object.
    type Evidence: PublicEvidence;

    /// Permanent positive number within the catalog.
    const NUMBER: CodeNumber;

    /// Short stable noun-like summary.
    const TITLE: &'static str;

    /// Safe default explanation suitable for a public response.
    const DETAIL: &'static str;

    /// Ordered actionable guidance for a caller or operator.
    const SUGGESTIONS: &'static [&'static str];

    /// Markdown documentation for the diagnostic.
    const DOCS: &'static str;
}
