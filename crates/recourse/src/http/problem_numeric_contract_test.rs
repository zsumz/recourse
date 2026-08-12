//! Governed numeric representation tests at the HTTP encoding boundary.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    catalog::{Catalog, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
};

use super::{
    Fixed, HttpProblemType, ProblemEncodeError,
    problem_contract_test::{TestCatalog, occurrence},
};

#[derive(Debug)]
struct OutOfRangeInt32Evidence;

impl Serialize for OutOfRangeInt32Evidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("answer", &2_147_483_648_u64)?;
        map.end()
    }
}

impl JsonSchema for OutOfRangeInt32Evidence {
    fn schema_name() -> Cow<'static, str> {
        "OutOfRangeInt32Evidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {"answer": {"type": "integer", "format": "int32"}},
            "required": ["answer"],
            "additionalProperties": false
        })
    }
}

impl PublicEvidence for OutOfRangeInt32Evidence {}

enum OutOfRangeInt32 {}

impl DiagnosticType for OutOfRangeInt32 {
    type Catalog = TestCatalog;
    type Evidence = OutOfRangeInt32Evidence;

    const NUMBER: CodeNumber = CodeNumber::new(4);
    const TITLE: &'static str = "Integer representation mismatch";
    const DETAIL: &'static str = "The serializer exceeds its governed integer representation.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Numeric representation conformance test.";
}

impl HttpProblemType for OutOfRangeInt32 {
    type Policy = Fixed<500>;
}

#[test]
fn governed_integer_bounds_reject_a_dishonest_serializer() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<OutOfRangeInt32>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"));
    let error = catalog
        .try_problem::<OutOfRangeInt32>(occurrence(), OutOfRangeInt32Evidence)
        .ok()
        .and_then(|problem| problem.try_encode().err());

    assert!(matches!(
        error,
        Some(ProblemEncodeError::EvidenceSchemaMismatch { path, .. }) if path == "$/answer"
    ));
}
