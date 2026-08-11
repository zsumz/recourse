//! External Ballast-shaped proof over extracted package artifacts.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
    routing::get,
};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    fault::PrivateReport,
    http::{Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter},
};
use recourse_axum::{HandlerResult, ProblemContext, RecourseLayer};
use schemars::JsonSchema;
use serde::Serialize;
use tower::ServiceExt;

const PRIVATE_CANARY: &str = "postgres://private-ballast-token";

#[derive(Debug)]
enum BallastCatalog {}

impl CatalogSpec for BallastCatalog {
    const NAME: &'static str = "ballast";
    const PREFIX: &'static str = "BAL";
    const TYPE_BASE: &'static str = "https://ballast.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct DeploymentEvidence {
    deployment_id: String,
}

impl PublicEvidence for DeploymentEvidence {}

#[derive(Debug)]
enum DeploymentNotFound {}

impl DiagnosticType for DeploymentNotFound {
    type Catalog = BallastCatalog;
    type Evidence = DeploymentEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1001);
    const TITLE: &'static str = "Deployment not found";
    const DETAIL: &'static str = "No deployment exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the deployment identifier."];
    const DOCS: &'static str = "Confirm the deployment exists before retrying.";
}

impl HttpProblemType for DeploymentNotFound {
    type Policy = Fixed<404>;
}

#[derive(Debug)]
enum InternalError {}

impl DiagnosticType for InternalError {
    type Catalog = BallastCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1002);
    const TITLE: &'static str = "Internal error";
    const DETAIL: &'static str = "Ballast could not complete the request.";
    const SUGGESTIONS: &'static [&'static str] = &["Retry the request later."];
    const DOCS: &'static str = "Contact support with the request ID.";
}

impl HttpProblemType for InternalError {
    type Policy = Fixed<500>;
}

#[derive(Debug)]
struct CanaryError;

impl Display for CanaryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(PRIVATE_CANARY)
    }
}

impl Error for CanaryError {}

#[derive(Debug, Clone, Default)]
struct Reports(Arc<Mutex<Vec<String>>>);

impl FaultReporter for Reports {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(report.to_string());
    }
}

async fn missing(problems: ProblemContext<BallastCatalog>) -> HandlerResult<&'static str> {
    Err(problems.problem::<DeploymentNotFound>(DeploymentEvidence {
        deployment_id: "dep_missing".to_owned(),
    }))
}

async fn fault(problems: ProblemContext<BallastCatalog>) -> HandlerResult<&'static str> {
    Err(problems.fault::<InternalError>(
        NoEvidence,
        PrivateReport::new(CanaryError).context("database", PRIVATE_CANARY),
    ))
}

fn app(reports: Reports) -> Result<Router, Box<dyn Error>> {
    let catalog = Catalog::<BallastCatalog>::builder()
        .problem::<DeploymentNotFound>()
        .problem::<InternalError>()
        .build()?;
    let layer = RecourseLayer::builder(catalog)
        .internal::<InternalError>()
        .fault_reporter(reports)
        .build()?;
    Ok(Router::new()
        .route("/deployments/{id}", get(missing))
        .route("/fault", get(fault))
        .layer(layer))
}

async fn assert_public_problem(app: Router) -> Result<(), Box<dyn Error>> {
    let request = Request::builder()
        .uri("/deployments/dep_missing")
        .header("x-request-id", "ballast-smoke-request")
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/problem+json");
    assert_eq!(response.headers()["x-request-id"], "ballast-smoke-request");
    let body = to_bytes(response.into_body(), 4096).await?;
    let problem: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(problem["type"], "https://ballast.invalid/problems/BAL-1001");
    assert_eq!(problem["code"], "BAL-1001");
    assert_eq!(problem["status"], 404);
    assert_eq!(problem["evidence"]["deployment_id"], "dep_missing");
    Ok(())
}

async fn assert_private_fault(app: Router, reports: &Reports) -> Result<(), Box<dyn Error>> {
    let request = Request::builder()
        .uri("/fault")
        .header("x-request-id", "ballast-fault-request")
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), 4096).await?;
    assert!(!String::from_utf8_lossy(&body).contains(PRIVATE_CANARY));
    let reports = reports
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].contains(PRIVATE_CANARY));
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let reports = Reports::default();
    let app = app(reports.clone())?;
    assert_public_problem(app.clone()).await?;
    assert_private_fault(app, &reports).await?;
    println!("packaged Ballast consumer passed");
    Ok(())
}
