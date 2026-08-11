//! Alternate-transport extraction from the concrete Axum failure.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    response::{IntoResponse, Response},
    routing::get,
};
use http::{Request, StatusCode};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
    observe::{HttpObserver, ProblemEvent},
};
use tower::ServiceExt;

use super::{ProblemContext, RecourseLayer};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "failure-extraction-test";
    const PREFIX: &'static str = "FET";
    const TYPE_BASE: &'static str = "https://failure.invalid/problems/";
}

macro_rules! diagnostic {
    ($name:ident, $number:literal, $status:literal) => {
        enum $name {}

        impl DiagnosticType for $name {
            type Catalog = TestCatalog;
            type Evidence = NoEvidence;

            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "Failure extraction test.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Failure extraction test.";
        }

        impl HttpProblemType for $name {
            type Policy = Fixed<$status>;
        }
    };
}

diagnostic!(Missing, 1, 404);
diagnostic!(Internal, 2, 500);

#[derive(Debug, Clone, Default)]
struct CountingObserver(Arc<AtomicUsize>);

impl HttpObserver for CountingObserver {
    fn on_problem(&self, _event: &ProblemEvent) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

impl CountingObserver {
    fn count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

async fn extracted(problems: ProblemContext<TestCatalog>) -> Response {
    let failure = problems.problem::<Missing>(NoEvidence);
    let Some(problem) = failure.into_encoded_problem() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let (status, headers, body) = problem.into_parts();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

#[tokio::test]
async fn consuming_failure_exposes_problem_and_delivers_observation_once() {
    let observer = CountingObserver::default();
    let catalog = Catalog::builder()
        .problem::<Missing>()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let layer = RecourseLayer::builder(catalog)
        .internal::<Internal>()
        .observer(observer.clone())
        .discard_faults()
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let request = Request::get("/events")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));

    let response = Router::new()
        .route("/events", get(extracted))
        .layer(layer)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("Problem body must be readable: {error}"));
    let body: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("Problem body must be JSON: {error}"));
    assert_eq!(body["code"], "FET-1");
    assert_eq!(observer.count(), 1);
}
