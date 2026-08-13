//! Exact integer spellings stay aligned across schema and catalog surfaces.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;
use serde_json::Value;

use crate::{
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    operation::OperationDiagnosticType,
};

use super::{
    Catalog, CatalogSpec, CodeNumber,
    schema::{self, number::values_equal},
};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "integer-emission-test";
    const PREFIX: &'static str = "IET";
    const TYPE_BASE: &'static str = "https://integer-emission.invalid/problems/";
}

#[test]
fn equivalent_integer_spellings_are_public_across_fixed_schema_keywords() {
    for encoded in [
        "9007199254740993.0",
        "18446744073709551615e0",
        "-9007199254740993.000",
    ] {
        assert_fixed_schema_keywords(encoded, "integer", true);
    }
}

#[test]
fn unavailable_numbers_remain_rejected_across_fixed_schema_keywords() {
    for encoded in ["18446744073709551616.0", "0.100000000000000000001", "1e400"] {
        assert_fixed_schema_keywords(encoded, "number", false);
    }
}

#[test]
fn equivalent_spellings_match_actual_primitive_serializer_output() {
    for (encoded, emitted) in [
        ("9007199254740993.0", Value::from(9_007_199_254_740_993_u64)),
        ("18446744073709551615e0", Value::from(u64::MAX)),
        (
            "-9007199254740993.000",
            Value::from(-9_007_199_254_740_993_i64),
        ),
    ] {
        let alternate: Value = serde_json::from_str(encoded)
            .unwrap_or_else(|error| panic!("alternate integer must parse: {error}"));
        assert!(values_equal(&alternate, &emitted), "{encoded}");
    }
}

fn assert_fixed_schema_keywords(encoded: &str, kind: &str, accepted: bool) {
    for constraints in [
        format!(r#"{{"type":"{kind}","const":{encoded}}}"#),
        format!(r#"{{"type":"{kind}","enum":[{encoded}]}}"#),
        format!(r#"{{"type":"{kind}","minimum":{encoded},"maximum":{encoded}}}"#),
    ] {
        let parsed: Value = serde_json::from_str(&constraints)
            .unwrap_or_else(|error| panic!("exact constraint must parse: {error}"));
        let mut schema_value = serde_json::json!({
            "type": "object",
            "properties": {"value": parsed},
            "required": ["value"],
            "additionalProperties": false
        });
        assert_eq!(
            schema::validate_artifact(&mut schema_value).is_ok(),
            accepted,
            "unexpected public-emitter result: {constraints}"
        );
    }
}

#[derive(Debug, Serialize)]
struct EquivalentIntegerImpact;

impl PublicEvidence for EquivalentIntegerImpact {}

impl JsonSchema for EquivalentIntegerImpact {
    fn schema_name() -> Cow<'static, str> {
        "EquivalentIntegerImpact".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        serde_json::from_str(
            r#"{
                "type":"object",
                "properties":{
                    "exact":{"type":"integer","const":9007199254740993.0},
                    "maximum":{"type":"integer","enum":[18446744073709551615e0]},
                    "negative":{
                        "type":"integer",
                        "minimum":-9007199254740993.000,
                        "maximum":-9007199254740993.000
                    }
                },
                "required":["exact","maximum","negative"],
                "additionalProperties":false
            }"#,
        )
        .unwrap_or_else(|error| panic!("equivalent integer impact must parse: {error}"))
    }
}

enum EquivalentIntegerDiagnostic {}

impl DiagnosticType for EquivalentIntegerDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Equivalent integer impact";
    const DETAIL: &'static str = "Equivalent integer spellings remain public.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Exact integer spellings are accepted.";
}

impl OperationDiagnosticType for EquivalentIntegerDiagnostic {
    type Impact = EquivalentIntegerImpact;
}

#[test]
fn operation_impact_accepts_equivalent_integer_spellings() {
    let catalog = Catalog::<TestCatalog>::builder()
        .operation::<EquivalentIntegerDiagnostic>()
        .build();

    assert!(catalog.is_ok());
}
