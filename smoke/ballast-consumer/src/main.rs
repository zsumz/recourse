//! Packaged Ballast-shaped HTTP fixture exercised externally by Smoque.

use std::{
    env,
    error::Error,
    fmt::{self, Display, Formatter},
};

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    fault::PrivateReport,
    http::{BasicChallenge, BasicUnauthorized, Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter, HttpObserver, ProblemEvent},
};
use recourse_axum::{HandlerResult, ProblemContext, RecourseLayer};
use schemars::JsonSchema;
use serde::Serialize;

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
enum RegistryAuthenticationRequired {}

impl DiagnosticType for RegistryAuthenticationRequired {
    type Catalog = BallastCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Registry authentication required";
    const DETAIL: &'static str = "Valid registry credentials are required.";
    const SUGGESTIONS: &'static [&'static str] = &["Provide registry credentials."];
    const DOCS: &'static str = "Authenticate with the registry token exchange.";
}

impl HttpProblemType for RegistryAuthenticationRequired {
    type Policy = BasicUnauthorized;
}

#[derive(Debug)]
struct CanaryError;

impl Display for CanaryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(PRIVATE_CANARY)
    }
}

impl Error for CanaryError {}

#[derive(Debug, Clone, Copy)]
struct ConsoleHooks;

impl HttpObserver for ConsoleHooks {
    fn on_problem(&self, event: &ProblemEvent) {
        println!("observed-problem: {}", event.code());
    }

    fn on_fault(&self, event: &FaultEvent) {
        println!("observed-fault: {}", event.problem_metadata().code());
    }
}

impl FaultReporter for ConsoleHooks {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        eprintln!("private-report: {report}");
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

async fn registry_token(
    problems: ProblemContext<BallastCatalog>,
) -> HandlerResult<&'static str> {
    let challenge = BasicChallenge::from_static("ballast-registry");
    Err(problems.problem_with::<RegistryAuthenticationRequired>(NoEvidence, challenge))
}

async fn stream_failure(problems: ProblemContext<BallastCatalog>) -> Response {
    let failure = problems.problem::<DeploymentNotFound>(DeploymentEvidence {
        deployment_id: "dep_stream".to_owned(),
    });
    let Some(problem) = failure.into_encoded_problem() else {
        return Response::new(Body::empty());
    };
    let (_, _, encoded) = problem.into_parts();
    let mut event = b"event: problem\ndata: ".to_vec();
    event.extend_from_slice(&encoded);
    event.extend_from_slice(b"\n\n");

    let mut response = Response::new(Body::from(event));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream; charset=utf-8"),
    );
    response
}

async fn ready() -> &'static str {
    "ready"
}

fn app() -> Result<Router, Box<dyn Error>> {
    let catalog = Catalog::<BallastCatalog>::builder()
        .problem::<DeploymentNotFound>()
        .problem::<InternalError>()
        .problem::<RegistryAuthenticationRequired>()
        .build()?;
    let layer = RecourseLayer::builder(catalog)
        .internal::<InternalError>()
        .observer(ConsoleHooks)
        .fault_reporter(ConsoleHooks)
        .build()?;
    Ok(Router::new()
        .route("/ready", get(ready))
        .route("/deployments/{id}", get(missing))
        .route("/fault", get(fault))
        .route("/registry/token", get(registry_token))
        .route("/stream", get(stream_failure))
        .layer(layer))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let port = env::var("PORT")?.parse::<u16>()?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    println!("ready on {port}");
    axum::serve(listener, app()?).await?;
    Ok(())
}
