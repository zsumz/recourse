//! Shared external-consumer fixture for dependency-feature determinism.

use std::{collections::HashMap, error::Error, io::Write};

use recourse::{
    catalog::{Catalog, CatalogLock, CatalogSpec, CodeNumber},
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

#[derive(Debug, Serialize, JsonSchema)]
struct ChangedEvidence {
    alpha: String,
    beta: String,
}

impl PublicEvidence for ChangedEvidence {}

enum ChangedProblem {}

impl DiagnosticType for ChangedProblem {
    type Catalog = ConsumerCatalog;
    type Evidence = ChangedEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Changed canonical output";
    const DETAIL: &'static str = "Compatibility ordering cannot depend on map features.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "External compatibility-order consumer fixture.";
}

impl HttpProblemType for ChangedProblem {
    type Policy = Fixed<500>;
}

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let catalog = Catalog::<ConsumerCatalog>::builder()
        .problem::<CanonicalProblem>()
        .build()?;
    let mut output = Vec::new();
    catalog.artifact().write_pretty(&mut output)?;
    output.extend_from_slice(b"--- lock from artifact ---\n");
    let lock = CatalogLock::from_artifact(&catalog.artifact());
    lock.write_pretty(&mut output)?;
    output.extend_from_slice(b"--- lock from slice ---\n");
    let parsed = parse_reordered_lock(&lock)?;
    parsed.write_pretty(&mut output)?;
    output.extend_from_slice(b"--- compatibility report ---\n");
    let changed = Catalog::<ConsumerCatalog>::builder()
        .problem::<ChangedProblem>()
        .build()?;
    serde_json::to_writer_pretty(&mut output, &parsed.check(&changed.artifact()))?;
    output.push(b'\n');
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

fn parse_reordered_lock(lock: &CatalogLock) -> Result<CatalogLock, Box<dyn Error>> {
    let mut value = serde_json::to_value(lock)?;
    let schema = value
        .pointer_mut("/entries/0/diagnostic/evidence_schema")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture lock is missing its evidence schema")?;
    let mut reversed = std::mem::take(schema).into_iter().collect::<Vec<_>>();
    reversed.sort_by(|left, right| right.0.cmp(&left.0));
    schema.extend(reversed);
    Ok(CatalogLock::from_slice(&serde_json::to_vec(&value)?)?)
}
