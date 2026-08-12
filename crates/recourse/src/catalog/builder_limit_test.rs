//! Catalog construction closes over envelope and artifact resource limits.

use std::{borrow::Cow, marker::PhantomData};

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{
    diagnostic::{DiagnosticType, PublicEvidence},
    http::{Fixed, HttpProblemType},
    wire::WireLimits,
};

use super::{
    Catalog, CatalogIssue, CatalogSpec, CodeNumber, ProblemSet,
    artifact::limits::MAX_ARTIFACT_STRING_BYTES,
};

enum LimitCatalog {}

impl CatalogSpec for LimitCatalog {
    const NAME: &'static str = "limit-catalog";
    const PREFIX: &'static str = "LIM";
    const TYPE_BASE: &'static str = "https://limit.invalid/problems/";
}

enum SchemaProblem<E> {
    _Marker(PhantomData<E>),
}

impl<E: PublicEvidence> DiagnosticType for SchemaProblem<E> {
    type Catalog = LimitCatalog;
    type Evidence = E;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Schema limit";
    const DETAIL: &'static str = "The schema fixture exercises catalog resource closure.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Catalog resource closure fixture.";
}

impl<E: PublicEvidence> HttpProblemType for SchemaProblem<E> {
    type Policy = Fixed<500>;
}

#[derive(Debug, Serialize)]
struct LongPropertyEvidence;

impl JsonSchema for LongPropertyEvidence {
    fn schema_name() -> Cow<'static, str> {
        "LongPropertyEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        object_schema(
            "x".repeat(MAX_ARTIFACT_STRING_BYTES + 1),
            json!({"type": "string"}),
        )
    }
}

impl PublicEvidence for LongPropertyEvidence {}

#[test]
fn builder_rejects_schema_property_names_beyond_artifact_limits() {
    let error = Catalog::<LimitCatalog>::builder()
        .problem::<SchemaProblem<LongPropertyEvidence>>()
        .build()
        .err();

    assert!(
        error.is_some_and(|error| error.issues().iter().any(|issue| matches!(
            issue,
            CatalogIssue::UnsupportedEvidenceSchema { reason, .. }
                if reason.contains("StringBytes")
        )))
    );
}

#[derive(Debug, Serialize)]
struct LongStringEvidence;

impl JsonSchema for LongStringEvidence {
    fn schema_name() -> Cow<'static, str> {
        "LongStringEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        object_schema(
            "value".to_owned(),
            json!({"enum": ["x".repeat(MAX_ARTIFACT_STRING_BYTES + 1)]}),
        )
    }
}

impl PublicEvidence for LongStringEvidence {}

#[test]
fn builder_rejects_schema_strings_beyond_artifact_limits() {
    let error = Catalog::<LimitCatalog>::builder()
        .problem::<SchemaProblem<LongStringEvidence>>()
        .build()
        .err();

    assert!(
        error.is_some_and(|error| error.issues().iter().any(|issue| matches!(
            issue,
            CatalogIssue::UnsupportedEvidenceSchema { reason, .. }
                if reason.contains("StringBytes")
        )))
    );
}

fn object_schema(name: String, property: Value) -> Schema {
    let mut properties = Map::new();
    properties.insert(name, property);
    let mut schema = Map::new();
    schema.insert("type".to_owned(), json!("object"));
    schema.insert("properties".to_owned(), Value::Object(properties));
    schema.into()
}

const LONG_TYPE_BASE_LEN: usize = WireLimits::DEFAULT_MAX_STRING_BYTES + 1;
const LONG_TYPE_BASE_BYTES: [u8; LONG_TYPE_BASE_LEN] = long_type_base();
const LONG_TYPE_BASE: &str = match std::str::from_utf8(&LONG_TYPE_BASE_BYTES) {
    Ok(value) => value,
    Err(_) => panic!("long type-base fixture must be UTF-8"),
};

const fn long_type_base() -> [u8; LONG_TYPE_BASE_LEN] {
    let mut value = [b'a'; LONG_TYPE_BASE_LEN];
    value[0] = b'h';
    value[1] = b't';
    value[2] = b't';
    value[3] = b'p';
    value[4] = b's';
    value[5] = b':';
    value[6] = b'/';
    value[7] = b'/';
    value[LONG_TYPE_BASE_LEN - 1] = b'/';
    value
}

enum LongTypeCatalog {}

impl CatalogSpec for LongTypeCatalog {
    const NAME: &'static str = "long-type";
    const PREFIX: &'static str = "LNG";
    const TYPE_BASE: &'static str = LONG_TYPE_BASE;
}

enum LongTypeProblem {}

impl DiagnosticType for LongTypeProblem {
    type Catalog = LongTypeCatalog;
    type Evidence = crate::diagnostic::NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Long type URI";
    const DETAIL: &'static str = "The derived type URI exceeds the wire profile.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Derived type URI limit fixture.";
}

impl HttpProblemType for LongTypeProblem {
    type Policy = Fixed<500>;
}

#[test]
fn builder_rejects_derived_type_uris_beyond_wire_limits() {
    let error = Catalog::<LongTypeCatalog>::builder()
        .problem::<LongTypeProblem>()
        .build()
        .err();

    assert!(
        error.is_some_and(|error| error.issues().iter().any(|issue| matches!(
            issue,
            CatalogIssue::TypeUriTooLong { maximum, .. }
                if *maximum == WireLimits::DEFAULT_MAX_STRING_BYTES
        )))
    );
}

#[test]
fn builder_rejects_pretty_artifacts_beyond_the_body_limit() {
    let mut builder = Catalog::<LimitCatalog>::builder()
        .problem::<SchemaProblem<crate::diagnostic::NoEvidence>>();
    for index in 0..70_000 {
        let id = format!("operation{index:05}{}", "x".repeat(104));
        builder = builder.problem_set(
            ProblemSet::builder(id)
                .include::<SchemaProblem<crate::diagnostic::NoEvidence>>()
                .build(),
        );
    }

    let error = builder.build().err();
    assert!(
        error.is_some_and(|error| error.issues().iter().any(|issue| matches!(
            issue,
            CatalogIssue::InvalidGeneratedArtifact { reason }
                if reason.contains("catalog artifact exceeds")
        )))
    );
}
