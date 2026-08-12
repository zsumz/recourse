//! Numeric identity regressions at the public Problem encoding boundary.

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
struct WideIntegerEvidence;

impl Serialize for WideIntegerEvidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("value", &u128::MAX)?;
        map.end()
    }
}

impl JsonSchema for WideIntegerEvidence {
    fn schema_name() -> Cow<'static, str> {
        "WideIntegerEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {"value": {"type": "integer"}},
            "required": ["value"],
            "additionalProperties": false
        })
    }
}

impl PublicEvidence for WideIntegerEvidence {}

enum WideIntegerDiagnostic {}

impl DiagnosticType for WideIntegerDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = WideIntegerEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(5);
    const TITLE: &'static str = "Wide integer";
    const DETAIL: &'static str = "Unformatted integers still preserve public JSON identity.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Numeric materialization regression.";
}

impl HttpProblemType for WideIntegerDiagnostic {
    type Policy = Fixed<500>;
}

#[derive(Debug, Serialize, JsonSchema)]
struct NullableFloatEvidence {
    float: Option<f32>,
    double: Option<f64>,
}

impl PublicEvidence for NullableFloatEvidence {}

enum NullableFloatDiagnostic {}

impl DiagnosticType for NullableFloatDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = NullableFloatEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(6);
    const TITLE: &'static str = "Nullable float";
    const DETAIL: &'static str = "Nonfinite values cannot become null.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Numeric materialization regression.";
}

impl HttpProblemType for NullableFloatDiagnostic {
    type Policy = Fixed<500>;
}

#[derive(Debug, Serialize, JsonSchema)]
struct FiniteFloatEvidence {
    value: f32,
}

impl PublicEvidence for FiniteFloatEvidence {}

enum FiniteFloatDiagnostic {}

impl DiagnosticType for FiniteFloatDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = FiniteFloatEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(7);
    const TITLE: &'static str = "Finite float";
    const DETAIL: &'static str = "Finite boundary values retain their public JSON value.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Numeric materialization regression.";
}

impl HttpProblemType for FiniteFloatDiagnostic {
    type Policy = Fixed<500>;
}

#[test]
fn rewritten_wide_and_nonfinite_numbers_fail_before_envelope_construction() {
    let wide_catalog = Catalog::<TestCatalog>::builder()
        .problem::<WideIntegerDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("wide fixture must build: {error}"));
    let wide = wide_catalog
        .try_problem::<WideIntegerDiagnostic>(occurrence(), WideIntegerEvidence)
        .ok()
        .and_then(|problem| problem.try_encode().err());
    assert!(matches!(
        wide,
        Some(ProblemEncodeError::EvidenceSerialization(_))
    ));

    let float_catalog = Catalog::<TestCatalog>::builder()
        .problem::<NullableFloatDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("float fixture must build: {error}"));
    for evidence in [
        NullableFloatEvidence {
            float: Some(f32::NAN),
            double: None,
        },
        NullableFloatEvidence {
            float: None,
            double: Some(f64::INFINITY),
        },
    ] {
        let error = float_catalog
            .try_problem::<NullableFloatDiagnostic>(occurrence(), evidence)
            .ok()
            .and_then(|problem| problem.try_encode().err());
        assert!(matches!(
            error,
            Some(ProblemEncodeError::EvidenceSerialization(_))
        ));
    }
}

#[test]
fn finite_f32_boundaries_cross_the_public_envelope_exactly() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<FiniteFloatDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("finite float fixture must build: {error}"));

    for value in [f32::MIN, f32::MAX] {
        let encoded = catalog
            .try_problem::<FiniteFloatDiagnostic>(occurrence(), FiniteFloatEvidence { value })
            .unwrap_or_else(|error| panic!("finite Problem must build: {error}"))
            .try_encode()
            .unwrap_or_else(|error| panic!("finite Problem must encode: {error}"));
        let body: serde_json::Value = serde_json::from_slice(encoded.body())
            .unwrap_or_else(|error| panic!("encoded Problem must parse: {error}"));
        let expected = serde_json::from_str::<serde_json::Value>(&value.to_string())
            .unwrap_or_else(|error| panic!("finite float fixture must parse: {error}"));
        assert_eq!(body.pointer("/evidence/value"), Some(&expected));
    }
}
