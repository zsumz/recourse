//! Encoding-fallback and post-response streaming boundary tests.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    response::Response,
    routing::get,
};
use futures_util::stream;
use http::{Request, StatusCode};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    fault::PrivateReport,
    http::{Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter, HttpObserver},
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer};
use tower::ServiceExt;

use super::{HandlerResult, ProblemContext, RecourseLayer};

enum FallbackCatalog {}

impl CatalogSpec for FallbackCatalog {
    const NAME: &'static str = "fallback-test";
    const PREFIX: &'static str = "FBT";
    const TYPE_BASE: &'static str = "https://fallback.invalid/problems/";
}

#[derive(Debug)]
struct DishonestEvidence;

impl Serialize for DishonestEvidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("not-an-object")
    }
}

impl JsonSchema for DishonestEvidence {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DishonestEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({"type": "object"})
    }
}

impl PublicEvidence for DishonestEvidence {}

macro_rules! diagnostic {
    ($name:ident, $number:literal, $evidence:ty, $status:literal) => {
        enum $name {}
        impl DiagnosticType for $name {
            type Catalog = FallbackCatalog;
            type Evidence = $evidence;
            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "Fallback boundary test.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Fallback boundary test.";
        }
        impl HttpProblemType for $name {
            type Policy = Fixed<$status>;
        }
    };
}

diagnostic!(DishonestProblem, 1, DishonestEvidence, 422);
diagnostic!(InternalProblem, 2, NoEvidence, 500);

#[derive(Debug, Default)]
struct Recorded {
    faults: Vec<(String, bool)>,
    reports: Vec<String>,
}

#[derive(Debug, Clone)]
struct RecordingPorts(Arc<Mutex<Recorded>>);

impl HttpObserver for RecordingPorts {
    fn on_fault(&self, event: &FaultEvent) {
        let metadata = event.problem_metadata();
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .faults
            .push((
                metadata.code().to_string(),
                metadata.used_fallback_encoding(),
            ));
    }
}

impl FaultReporter for RecordingPorts {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .reports
            .push(report.to_string());
    }
}

fn app(recorded: &Arc<Mutex<Recorded>>) -> Router {
    let catalog = Catalog::<FallbackCatalog>::builder()
        .problem::<DishonestProblem>()
        .problem::<InternalProblem>()
        .build()
        .unwrap_or_else(|error| panic!("fallback catalog must build: {error}"));
    let ports = RecordingPorts(Arc::clone(recorded));
    let layer = RecourseLayer::builder(catalog)
        .internal::<InternalProblem>()
        .observer(ports.clone())
        .fault_reporter(ports)
        .build()
        .unwrap_or_else(|error| panic!("fallback layer must build: {error}"));
    Router::new()
        .route("/dishonest", get(dishonest))
        .route("/stream", get(failing_stream))
        .layer(layer)
}

async fn dishonest(problems: ProblemContext<FallbackCatalog>) -> HandlerResult<&'static str> {
    Err(problems.problem::<DishonestProblem>(DishonestEvidence))
}

async fn failing_stream() -> Response {
    let body = Body::from_stream(stream::once(async { Err::<Bytes, _>(CanaryStreamError) }));
    Response::new(body)
}

#[derive(Debug)]
struct CanaryStreamError;

impl Display for CanaryStreamError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PRIVATE_STREAM_CANARY")
    }
}

impl Error for CanaryStreamError {}

#[tokio::test]
async fn evidence_encoding_failure_becomes_the_sanitized_internal_problem() {
    let recorded = Arc::new(Mutex::new(Recorded::default()));
    let request = Request::get("/dishonest")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("dishonest request must build: {error}"));
    let response = app(&recorded)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("fallback body must be readable: {error}"));
    let body: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("fallback body must be JSON: {error}"));
    assert_eq!(body["code"], "FBT-2");

    let recorded = recorded
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(recorded.faults, [("FBT-2".to_owned(), true)]);
    assert_eq!(recorded.reports.len(), 1);
}

#[tokio::test]
async fn a_body_stream_failure_is_not_replaced_after_the_response_exists() {
    let recorded = Arc::new(Mutex::new(Recorded::default()));
    let request = Request::get("/stream")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("stream request must build: {error}"));
    let response = app(&recorded)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});
    assert_eq!(response.status(), StatusCode::OK);
    assert!(to_bytes(response.into_body(), 4096).await.is_err());

    let recorded = recorded
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(recorded.faults.is_empty());
    assert!(recorded.reports.is_empty());
}
