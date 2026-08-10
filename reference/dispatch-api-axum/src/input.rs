//! Request representation and semantic validation translation.

use axum::{
    body::Bytes,
    http::{HeaderMap, header::CONTENT_TYPE},
};
use dispatch_diagnostics::{
    DispatchCatalog, InternalError, MalformedRequest, UnsupportedMediaType, ValidationFailed,
};
use dispatch_model::{CreateJobRequest, Destination, IdempotencyKey};
use recourse::{
    diagnostic::{NoEvidence, PublicText},
    fault::PrivateReport,
    validation::{
        HeaderName, JsonPointer, ValidationEvidence, Violation, ViolationReason, ViolationSource,
    },
};
use recourse_axum::{HttpFailure, ProblemContext};

const IDEMPOTENCY_KEY: &str = "idempotency-key";
const IDEMPOTENCY_FIELD: HeaderName = HeaderName::from_static(IDEMPOTENCY_KEY);
const IDEMPOTENCY_DETAIL: PublicText =
    PublicText::from_static("Provide a visible-ASCII Idempotency-Key of at most 128 bytes.");
const DESTINATION_POINTER: JsonPointer = JsonPointer::from_static("/destination");
const DESTINATION_DETAIL: PublicText =
    PublicText::from_static("Provide a nonempty destination of at most 256 bytes.");

pub(crate) fn create_job(
    headers: &HeaderMap,
    body: &Bytes,
    problems: &ProblemContext<DispatchCatalog>,
) -> Result<(IdempotencyKey, CreateJobRequest), HttpFailure> {
    require_json(headers, problems)?;
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| problems.problem::<MalformedRequest>(NoEvidence))?;
    let mut violations = Vec::new();
    let destination = destination(&value, &mut violations);
    let key = idempotency_key(headers, &mut violations);
    if !violations.is_empty() {
        let evidence = ValidationEvidence::new(violations).map_err(|error| {
            problems.fault::<InternalError>(
                NoEvidence,
                PrivateReport::new(error).context("operation", "build_validation_evidence"),
            )
        })?;
        return Err(problems.problem::<ValidationFailed>(evidence));
    }
    match (key, destination) {
        (Some(key), Some(destination)) => Ok((key, CreateJobRequest { destination })),
        _ => Err(problems.fault::<InternalError>(
            NoEvidence,
            PrivateReport::new(InputInvariant)
                .context("operation", "validated_create_input_missing"),
        )),
    }
}

fn require_json(
    headers: &HeaderMap,
    problems: &ProblemContext<DispatchCatalog>,
) -> Result<(), HttpFailure> {
    let is_json = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if is_json {
        Ok(())
    } else {
        Err(problems.problem::<UnsupportedMediaType>(NoEvidence))
    }
}

fn destination(value: &serde_json::Value, violations: &mut Vec<Violation>) -> Option<Destination> {
    let raw = value
        .as_object()
        .and_then(|object| object.get("destination"));
    match raw {
        Some(serde_json::Value::String(value)) => {
            if let Ok(destination) = Destination::new(value) {
                return Some(destination);
            }
            violations.push(body_violation(ViolationReason::OutOfRange));
            None
        }
        None => {
            violations.push(body_violation(ViolationReason::Required));
            None
        }
        Some(_) => {
            violations.push(body_violation(ViolationReason::InvalidFormat));
            None
        }
    }
}

fn idempotency_key(headers: &HeaderMap, violations: &mut Vec<Violation>) -> Option<IdempotencyKey> {
    let raw = headers
        .get(IDEMPOTENCY_KEY)
        .and_then(|value| value.to_str().ok());
    match raw.map(IdempotencyKey::new) {
        Some(Ok(key)) => Some(key),
        Some(Err(_)) => {
            violations.push(header_violation(ViolationReason::InvalidFormat));
            None
        }
        None => {
            violations.push(header_violation(ViolationReason::Required));
            None
        }
    }
}

fn body_violation(reason: ViolationReason) -> Violation {
    Violation {
        reason,
        detail: DESTINATION_DETAIL,
        source: ViolationSource::Body {
            pointer: DESTINATION_POINTER,
        },
    }
}

fn header_violation(reason: ViolationReason) -> Violation {
    Violation {
        reason,
        detail: IDEMPOTENCY_DETAIL,
        source: ViolationSource::Header {
            name: IDEMPOTENCY_FIELD,
        },
    }
}

#[derive(Debug)]
struct InputInvariant;

impl std::fmt::Display for InputInvariant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("validated create input is incomplete")
    }
}

impl std::error::Error for InputInvariant {}
