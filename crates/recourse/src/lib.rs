//! Framework-neutral diagnostic identity, envelopes, and governance.
//!
//! Recourse applications declare stable diagnostic marker types, register them
//! explicitly, and construct protocol values through the resulting catalog.
//!
//! Public signatures name types from `http` 1, `schemars` 1, and `serde` 1;
//! [`dependencies`] re-exports each crate at that required major version.
//!
//! ```
//! use recourse::{
//!     catalog::{Catalog, CatalogSpec, CodeNumber},
//!     diagnostic::{DiagnosticType, NoEvidence},
//!     http::{CorrelationId, Fixed, HttpProblemType, ProblemOccurrence},
//! };
//!
//! enum ServiceCatalog {}
//!
//! impl CatalogSpec for ServiceCatalog {
//!     const NAME: &'static str = "example-service";
//!     const PREFIX: &'static str = "EXM";
//!     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
//! }
//!
//! enum ResourceMissing {}
//!
//! impl DiagnosticType for ResourceMissing {
//!     type Catalog = ServiceCatalog;
//!     type Evidence = NoEvidence;
//!
//!     const NUMBER: CodeNumber = CodeNumber::new(1001);
//!     const TITLE: &'static str = "Resource missing";
//!     const DETAIL: &'static str = "The requested resource does not exist.";
//!     const SUGGESTIONS: &'static [&'static str] = &["Check the resource identifier."];
//!     const DOCS: &'static str = "Verify the identifier before retrying.";
//! }
//!
//! impl HttpProblemType for ResourceMissing {
//!     type Policy = Fixed<404>;
//! }
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let catalog = Catalog::<ServiceCatalog>::builder()
//!     .problem::<ResourceMissing>()
//!     .build()?;
//! let occurrence = ProblemOccurrence::new(
//!     CorrelationId::new("request-01")?,
//!     "/problem-occurrences/request-01",
//! )?;
//! let encoded = catalog
//!     .try_problem::<ResourceMissing>(occurrence, NoEvidence)?
//!     .try_encode()?;
//!
//! assert_eq!(encoded.status().as_u16(), 404);
//! assert_eq!(encoded.headers()["content-type"], "application/problem+json");
//! # Ok(())
//! # }
//! # assert!(example().is_ok());
//! ```

#![deny(missing_docs)]

mod materialize;

pub mod catalog;
pub mod client;
pub mod dependencies;
pub mod diagnostic;
pub mod fault;
pub mod health;
pub mod http;
pub mod observe;
pub mod operation;
pub mod validation;
pub mod wire;

#[cfg(test)]
mod materialize_test;
