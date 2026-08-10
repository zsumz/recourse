//! Focused and adversarial tests for Problem occurrence identity.

use super::{CorrelationId, CorrelationIdError, ProblemOccurrence, ProblemOccurrenceError};

#[test]
fn correlation_id_is_safe_for_header_echo() {
    let id = CorrelationId::new("01K00000000000000000000000");

    assert_eq!(
        id.as_ref().map(CorrelationId::as_str),
        Ok("01K00000000000000000000000")
    );
    assert_eq!(CorrelationId::new(""), Err(CorrelationIdError::Empty));
    assert!(matches!(
        CorrelationId::new("two words"),
        Err(CorrelationIdError::InvalidByte { byte_index: 3, .. })
    ));
    assert!(matches!(
        CorrelationId::new("é"),
        Err(CorrelationIdError::InvalidByte { byte_index: 0, .. })
    ));
}

#[test]
fn occurrence_accepts_absolute_and_relative_instances() {
    let Some(id) = CorrelationId::new("request-1").ok() else {
        return;
    };
    let absolute = ProblemOccurrence::new(
        id.clone(),
        "https://api.dispatch.invalid/problem-occurrences/request-1",
    );
    let relative = ProblemOccurrence::new(id, "/problem-occurrences/request-1");

    assert!(absolute.is_ok());
    assert_eq!(
        relative.as_ref().map(|value| value.instance().to_string()),
        Ok("/problem-occurrences/request-1".to_owned())
    );
}

#[test]
fn invalid_instance_references_are_rejected() {
    let Some(id) = CorrelationId::new("request-1").ok() else {
        return;
    };

    assert_eq!(
        ProblemOccurrence::new(id.clone(), ""),
        Err(ProblemOccurrenceError::InvalidInstance)
    );
    assert_eq!(
        ProblemOccurrence::new(id, "contains a space"),
        Err(ProblemOccurrenceError::InvalidInstance)
    );
}
