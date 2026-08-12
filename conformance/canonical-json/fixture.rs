//! Shared external-consumer fixture for dependency-feature determinism.

use std::{collections::HashMap, error::Error, io::Write};

use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
    http::{CorrelationId, Fixed, HttpProblemType, ProblemOccurrence},
};
use schemars::JsonSchema;
use serde::Serialize;

enum ConsumerCatalog {}

impl CatalogSpec for ConsumerCatalog {
    const NAME: &'static str = "canonical-consumer";
    const PREFIX: &'static str = "CAN";
    const TYPE_BASE: &'static str = "https://canonical.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct Evidence {
    zeta: String,
    alpha: String,
    labels: HashMap<String, String>,
}

impl PublicEvidence for Evidence {}

enum CanonicalProblem {}

impl DiagnosticType for CanonicalProblem {
    type Catalog = ConsumerCatalog;
    type Evidence = Evidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Canonical output";
    const DETAIL: &'static str = "Dependency features cannot change these bytes.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "External canonical-output consumer fixture.";
}

impl HttpProblemType for CanonicalProblem {
    type Policy = Fixed<500>;
}

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let catalog = Catalog::<ConsumerCatalog>::builder()
        .problem::<CanonicalProblem>()
        .build()?;
    let mut output = Vec::new();
    catalog.artifact().write_pretty(&mut output)?;
    output.extend_from_slice(b"--- problem ---\n");
    let occurrence = ProblemOccurrence::new(
        CorrelationId::new("canonical-01")?,
        "/problem-occurrences/canonical-01",
    )?;
    let mut labels = HashMap::new();
    labels.insert("zeta".to_owned(), "last".to_owned());
    labels.insert("alpha".to_owned(), "first".to_owned());
    let encoded = catalog
        .try_problem::<CanonicalProblem>(
            occurrence,
            Evidence {
                zeta: "last".to_owned(),
                alpha: "first".to_owned(),
                labels,
            },
        )?
        .try_encode()?;
    output.extend_from_slice(encoded.body());
    output.push(b'\n');
    std::io::stdout().write_all(&output)?;
    Ok(())
}
