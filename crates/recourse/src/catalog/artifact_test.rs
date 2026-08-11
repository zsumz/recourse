//! Exact fixture tests for deterministic catalog artifact output.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogSpec, CodeNumber};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the job identifier."];
    const DOCS: &'static str = "The job identifier is unknown to Dispatch.";
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}

enum CheckoutStableDocumentation {}

impl DiagnosticType for CheckoutStableDocumentation {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1004);
    const TITLE: &'static str = "Checkout-stable documentation";
    const DETAIL: &'static str = "Catalog documentation has canonical line endings.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "First paragraph.\r\n\r\nSecond paragraph.\r";
}

impl HttpProblemType for CheckoutStableDocumentation {
    type Policy = Fixed<500>;
}

#[test]
fn pretty_artifact_matches_the_canonical_wire_fixture() {
    let catalog = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .build();
    let Some(catalog) = catalog.ok() else {
        return;
    };
    let mut output = Vec::new();
    assert!(catalog.artifact().write_pretty(&mut output).is_ok());
    let output = String::from_utf8(output);
    let expected = r#"{
  "schema_version": 1,
  "catalog": {
    "name": "dispatch",
    "prefix": "DSP",
    "type_base": "https://dispatch.invalid/problems/"
  },
  "diagnostics": [
    {
      "number": 1003,
      "code": "DSP-1003",
      "type": "https://dispatch.invalid/problems/DSP-1003",
      "title": "Job not found",
      "detail": "No job exists for the supplied identifier.",
      "suggestions": [
        "Check the job identifier."
      ],
      "documentation_markdown": "The job identifier is unknown to Dispatch.",
      "evidence_schema": {
        "type": "object"
      },
      "surfaces": {
        "http": {
          "status": 404,
          "policy": "fixed",
          "required_headers": []
        }
      }
    }
  ],
  "problem_sets": {}
}
"#;

    assert!(matches!(output, Ok(value) if value == expected));
}

#[test]
fn authored_markdown_has_platform_neutral_line_endings() {
    let catalog = Catalog::<DispatchCatalog>::builder()
        .problem::<CheckoutStableDocumentation>()
        .build();
    let Some(catalog) = catalog.ok() else {
        return;
    };

    assert_eq!(
        catalog.artifact().diagnostics()[0].documentation_markdown(),
        "First paragraph.\n\nSecond paragraph.\n"
    );
}
