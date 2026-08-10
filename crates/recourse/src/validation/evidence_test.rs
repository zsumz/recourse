//! Exact and adversarial tests for structured validation evidence.

use crate::diagnostic::PublicText;

use super::{
    JsonPointer, ParameterName, ValidationEvidence, ValidationEvidenceError, Violation,
    ViolationReason, ViolationSource,
};

fn required_destination() -> Option<Violation> {
    Some(Violation {
        reason: ViolationReason::Required,
        detail: PublicText::new("destination is required").ok()?,
        source: ViolationSource::Body {
            pointer: JsonPointer::new("/destination").ok()?,
        },
    })
}

#[test]
fn validation_evidence_matches_the_public_wire_shape() {
    let Some(violation) = required_destination() else {
        return;
    };
    let evidence = ValidationEvidence::new(vec![violation]);
    let Some(evidence) = evidence.ok() else {
        return;
    };

    assert!(matches!(
        serde_json::to_value(evidence),
        Ok(value) if value == serde_json::json!({
            "violations": [{
                "reason": "required",
                "detail": "destination is required",
                "source": { "body": { "pointer": "/destination" } }
            }]
        })
    ));
}

#[test]
fn every_location_variant_omits_input_values() {
    let Some(priority) = ParameterName::new("priority").ok() else {
        return;
    };
    let Some(job_id) = ParameterName::new("job_id").ok() else {
        return;
    };
    let query = ViolationSource::Query {
        parameter: priority,
    };
    let path = ViolationSource::Path { parameter: job_id };

    assert!(matches!(
        serde_json::to_value(query),
        Ok(value) if value == serde_json::json!({"query": {"parameter": "priority"}})
    ));
    assert!(matches!(
        serde_json::to_value(path),
        Ok(value) if value == serde_json::json!({"path": {"parameter": "job_id"}})
    ));
}

#[test]
fn evidence_rejects_empty_and_excessive_collections() {
    assert_eq!(
        ValidationEvidence::new(Vec::new()),
        Err(ValidationEvidenceError::Empty)
    );
    let Some(violation) = required_destination() else {
        return;
    };
    assert!(matches!(
        ValidationEvidence::with_max_violations(vec![violation.clone(), violation], 1),
        Err(ValidationEvidenceError::TooMany {
            actual: 2,
            max_violations: 1
        })
    ));
    assert!(matches!(
        ValidationEvidence::with_max_violations(Vec::new(), 101),
        Err(ValidationEvidenceError::LimitTooLarge {
            max_violations: 101
        })
    ));
}

#[test]
fn decoding_reapplies_collection_bounds() {
    assert!(serde_json::from_str::<ValidationEvidence>(r#"{"violations":[]}"#).is_err());
}
