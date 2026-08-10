//! Typed public impact contract for durable operation failures.

use crate::diagnostic::{DiagnosticType, PublicEvidence};

/// Declares a diagnostic usable after accepted work fails durably.
///
/// Impact must cross the same explicit public-data boundary as evidence:
///
/// ```compile_fail
/// use recourse::{
///     catalog::{CatalogSpec, CodeNumber},
///     diagnostic::{DiagnosticType, NoEvidence},
///     operation::OperationDiagnosticType,
/// };
///
/// enum ExampleCatalog {}
/// impl CatalogSpec for ExampleCatalog {
///     const NAME: &'static str = "example";
///     const PREFIX: &'static str = "EXM";
///     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
/// }
///
/// enum UnreviewedOperation {}
/// impl DiagnosticType for UnreviewedOperation {
///     type Catalog = ExampleCatalog;
///     type Evidence = NoEvidence;
///     const NUMBER: CodeNumber = CodeNumber::new(1);
///     const TITLE: &'static str = "Unreviewed operation";
///     const DETAIL: &'static str = "Its impact was not reviewed.";
///     const SUGGESTIONS: &'static [&'static str] = &[];
///     const DOCS: &'static str = "Review the impact type.";
/// }
/// impl OperationDiagnosticType for UnreviewedOperation {
///     type Impact = String;
/// }
/// ```
pub trait OperationDiagnosticType: DiagnosticType {
    /// Reviewed caller-visible facts about what the failed work changed.
    type Impact: PublicEvidence;
}
