//! Arbitrary-precision number tokens cannot bypass governed evidence serialization.

use std::{borrow::Cow, error::Error};

use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
    http::{CorrelationId, Fixed, HttpProblemType, ProblemEncodeError, ProblemOccurrence},
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde_json::Number;

enum ConsumerCatalog {}

impl CatalogSpec for ConsumerCatalog {
    const NAME: &'static str = "arbitrary-number-consumer";
    const PREFIX: &'static str = "ARB";
    const TYPE_BASE: &'static str = "https://arbitrary.invalid/problems/";
}

#[derive(Debug, Serialize)]
struct NumberEvidence {
    value: Number,
}

impl JsonSchema for NumberEvidence {
    fn schema_name() -> Cow<'static, str> {
        "NumberEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {"value": {"type": "number"}},
            "required": ["value"],
            "additionalProperties": false
        })
    }
}

impl PublicEvidence for NumberEvidence {}

enum NumberProblem {}

impl DiagnosticType for NumberProblem {
    type Catalog = ConsumerCatalog;
    type Evidence = NumberEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Arbitrary JSON number";
    const DETAIL: &'static str = "Arbitrary JSON numbers are outside the evidence profile.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "External arbitrary-precision feature-unification fixture.";
}

impl HttpProblemType for NumberProblem {
    type Policy = Fixed<500>;
}

fn main() -> Result<(), Box<dyn Error>> {
    let catalog = Catalog::<ConsumerCatalog>::builder()
        .problem::<NumberProblem>()
        .build()?;
    let value = serde_json::from_str::<Number>("18446744073709551616")?;
    let result = catalog
        .try_problem::<NumberProblem>(occurrence()?, NumberEvidence { value })?
        .try_encode();
    if !matches!(result, Err(ProblemEncodeError::EvidenceSerialization(_))) {
        return Err(format!("arbitrary-precision evidence was not rejected: {result:?}").into());
    }
    Ok(())
}

fn occurrence() -> Result<ProblemOccurrence, Box<dyn Error>> {
    Ok(ProblemOccurrence::new(
        CorrelationId::new("arbitrary-number-01")?,
        "/problem-occurrences/arbitrary-number-01",
    )?)
}
