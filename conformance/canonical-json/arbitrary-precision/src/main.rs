//! Arbitrary-precision number tokens cannot bypass governed evidence serialization.

use std::{borrow::Cow, error::Error};

use recourse::{
    catalog::{Catalog, CatalogArtifact, CatalogSpec, CodeNumber},
    client::{DecodeLimits, ReceivedProblem},
    dependencies::http::{HeaderMap, StatusCode},
    diagnostic::{DiagnosticType, PublicEvidence},
    http::{CorrelationId, Fixed, HttpProblemType, ProblemEncodeError, ProblemOccurrence},
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
struct FloatEvidence {
    ratio: f64,
    #[serde(default)]
    single: Option<f32>,
}

impl PublicEvidence for FloatEvidence {}

enum FloatProblem {}

impl DiagnosticType for FloatProblem {
    type Catalog = ConsumerCatalog;
    type Evidence = FloatEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(2);
    const TITLE: &'static str = "Floating-point evidence";
    const DETAIL: &'static str = "Finite floating-point evidence must survive decoding.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "External arbitrary-precision decoding fixture.";
}

impl HttpProblemType for FloatProblem {
    type Policy = Fixed<400>;
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
struct CountEvidence {
    count: u128,
}

impl PublicEvidence for CountEvidence {}

enum CountProblem {}

impl DiagnosticType for CountProblem {
    type Catalog = ConsumerCatalog;
    type Evidence = CountEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(3);
    const TITLE: &'static str = "Wide count evidence";
    const DETAIL: &'static str = "Wide integer evidence must survive tolerant decoding.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "External arbitrary-precision decoding fixture.";
}

impl HttpProblemType for CountProblem {
    type Policy = Fixed<400>;
}

fn main() -> Result<(), Box<dyn Error>> {
    outbound_arbitrary_numbers_fail_closed()?;
    inbound_numbers_retain_their_wire_identity()?;
    parser_private_keys_remain_caller_objects()?;
    duplicate_members_still_fail_closed()?;
    float_schemas_survive_catalog_reparsing()?;
    Ok(())
}

fn outbound_arbitrary_numbers_fail_closed() -> Result<(), Box<dyn Error>> {
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

fn inbound_numbers_retain_their_wire_identity() -> Result<(), Box<dyn Error>> {
    for token in ["1.25", "1e-30"] {
        let body = format!(
            r#"{{"type":"https://arbitrary.invalid/problems/ARB-2","code":"ARB-2","status":400,"evidence":{{"ratio":{token}}}}}"#
        );
        let problem = received(body.as_bytes())?;
        assert_number_round_trip(&problem, "ratio", token)?;
        let typed = problem
            .try_as::<FloatProblem>()?
            .ok_or("float Problem did not match its declaration")?
            .evidence()?;
        let expected = token.parse::<f64>()?;
        if typed.ratio.to_bits() != expected.to_bits() {
            return Err(format!("typed ratio changed {token}: {}", typed.ratio).into());
        }
    }

    let token = "18446744073709551616";
    let body = format!(
        r#"{{"type":"https://arbitrary.invalid/problems/ARB-3","code":"ARB-3","status":400,"evidence":{{"count":{token}}}}}"#
    );
    let problem = received(body.as_bytes())?;
    assert_number_round_trip(&problem, "count", token)?;
    let typed = problem
        .try_as::<CountProblem>()?
        .ok_or("count Problem did not match its declaration")?
        .evidence()?;
    if typed.count != 18_446_744_073_709_551_616_u128 {
        return Err(format!("typed count changed: {}", typed.count).into());
    }
    Ok(())
}

fn parser_private_keys_remain_caller_objects() -> Result<(), Box<dyn Error>> {
    let body = br#"{"evidence":{"opaque":{"$serde_json::private::Number":"1.25"}}}"#;
    let problem = received(body)?;
    if problem.raw()["evidence"]["opaque"]["$serde_json::private::Number"] != "1.25" {
        return Err("caller object was mistaken for a parser-generated number".into());
    }
    Ok(())
}

fn duplicate_members_still_fail_closed() -> Result<(), Box<dyn Error>> {
    let body = br#"{"evidence":{"ratio":1.25,"ratio":1e-30}}"#;
    let error = ReceivedProblem::from_slice(
        StatusCode::BAD_REQUEST,
        &HeaderMap::new(),
        body,
        DecodeLimits::default(),
    )
    .err()
    .ok_or("duplicate evidence member was accepted")?;
    if !error.to_string().contains("duplicate JSON member") {
        return Err(format!("duplicate failed for the wrong reason: {error}").into());
    }
    Ok(())
}

fn float_schemas_survive_catalog_reparsing() -> Result<(), Box<dyn Error>> {
    let artifact = Catalog::<ConsumerCatalog>::builder()
        .problem::<FloatProblem>()
        .build()?
        .artifact();
    let mut body = Vec::new();
    artifact.write_pretty(&mut body)?;
    let reparsed = CatalogArtifact::from_slice(&body)?;
    if reparsed != artifact {
        return Err("float-bearing catalog changed while reparsing".into());
    }
    Ok(())
}

fn received(body: &[u8]) -> Result<ReceivedProblem, Box<dyn Error>> {
    Ok(ReceivedProblem::from_slice(
        StatusCode::BAD_REQUEST,
        &HeaderMap::new(),
        body,
        DecodeLimits::default(),
    )?)
}

fn assert_number_round_trip(
    problem: &ReceivedProblem,
    member: &str,
    expected: &str,
) -> Result<(), Box<dyn Error>> {
    let number = problem.raw()["evidence"][member]
        .as_number()
        .ok_or("wire number was not retained as a JSON number")?;
    if number.to_string() != expected {
        return Err(format!("wire number changed: expected {expected}, found {number}").into());
    }
    let encoded = serde_json::to_vec(problem.raw())?;
    let reparsed = serde_json::from_slice::<serde_json::Value>(&encoded)?;
    if reparsed["evidence"][member].as_number() != Some(number) {
        return Err(format!("wire number changed while re-encoding {expected}").into());
    }
    Ok(())
}

fn occurrence() -> Result<ProblemOccurrence, Box<dyn Error>> {
    Ok(ProblemOccurrence::new(
        CorrelationId::new("arbitrary-number-01")?,
        "/problem-occurrences/arbitrary-number-01",
    )?)
}
