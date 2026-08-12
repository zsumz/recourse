//! Raw JSON tokens cannot bypass governed evidence serialization.

use std::{borrow::Cow, error::Error};

use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
    http::{CorrelationId, Fixed, HttpProblemType, ProblemEncodeError, ProblemOccurrence},
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer, ser::SerializeStruct};
use serde_json::value::RawValue;

const RAW_TOKEN: &str = "$serde_json::private::RawValue";

enum ConsumerCatalog {}

impl CatalogSpec for ConsumerCatalog {
    const NAME: &'static str = "raw-value-consumer";
    const PREFIX: &'static str = "RAW";
    const TYPE_BASE: &'static str = "https://raw.invalid/problems/";
}

#[derive(Debug)]
enum RawPayload {
    Safe(Box<RawValue>),
    Spoof(&'static str),
    NewtypeSpoof(&'static str),
}

impl Serialize for RawPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Safe(value) => value.serialize(serializer),
            Self::Spoof(value) => {
                let mut raw = serializer.serialize_struct(RAW_TOKEN, 1)?;
                raw.serialize_field(RAW_TOKEN, value)?;
                raw.end()
            }
            Self::NewtypeSpoof(value) => serializer.serialize_newtype_struct(RAW_TOKEN, value),
        }
    }
}

#[derive(Debug, Serialize)]
struct RawEvidence {
    value: RawPayload,
}

impl JsonSchema for RawEvidence {
    fn schema_name() -> Cow<'static, str> {
        "RawEvidence".into()
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

impl PublicEvidence for RawEvidence {}

enum RawProblem {}

impl DiagnosticType for RawProblem {
    type Catalog = ConsumerCatalog;
    type Evidence = RawEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Raw JSON evidence";
    const DETAIL: &'static str = "Raw JSON tokens are outside the evidence profile.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "External raw-value feature-unification fixture.";
}

impl HttpProblemType for RawProblem {
    type Policy = Fixed<500>;
}

fn main() -> Result<(), Box<dyn Error>> {
    let catalog = Catalog::<ConsumerCatalog>::builder()
        .problem::<RawProblem>()
        .build()?;
    let wider_than_u64 = "18446744073709551616";
    let over_precise = "0.123456789012345678901234567890123456789";

    for payload in [
        RawPayload::Safe(RawValue::from_string(wider_than_u64.to_owned())?),
        RawPayload::Safe(RawValue::from_string(over_precise.to_owned())?),
        RawPayload::Spoof(wider_than_u64),
        RawPayload::NewtypeSpoof(wider_than_u64),
    ] {
        let result = catalog
            .try_problem::<RawProblem>(occurrence()?, RawEvidence { value: payload })?
            .try_encode();
        if !matches!(result, Err(ProblemEncodeError::EvidenceSerialization(_))) {
            return Err(format!("raw JSON evidence was not rejected: {result:?}").into());
        }
    }
    Ok(())
}

fn occurrence() -> Result<ProblemOccurrence, Box<dyn Error>> {
    Ok(ProblemOccurrence::new(
        CorrelationId::new("raw-value-01")?,
        "/problem-occurrences/raw-value-01",
    )?)
}
