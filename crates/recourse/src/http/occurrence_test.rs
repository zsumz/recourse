//! Focused and adversarial tests for Problem occurrence identity.

use super::{
    CorrelationId, CorrelationIdError, ProblemOccurrence, ProblemOccurrenceError, UriReferenceError,
};

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
    let rootless = ProblemOccurrence::new(id.clone(), "occurrence-123");
    let fragmented = ProblemOccurrence::new(id, "/problem-occurrences/request-1#attempt-2");

    assert!(absolute.is_ok());
    assert_eq!(
        rootless.as_ref().map(|value| value.instance().as_str()),
        Ok("occurrence-123")
    );
    assert_eq!(
        fragmented.as_ref().map(|value| value.instance().as_str()),
        Ok("/problem-occurrences/request-1#attempt-2")
    );
}

#[test]
fn invalid_instance_references_are_rejected() {
    let Some(id) = CorrelationId::new("request-1").ok() else {
        return;
    };

    assert_eq!(
        ProblemOccurrence::new(id.clone(), ""),
        Err(ProblemOccurrenceError::InvalidInstance(
            UriReferenceError::Empty
        ))
    );
    assert!(matches!(
        ProblemOccurrence::new(id, "contains a space"),
        Err(ProblemOccurrenceError::InvalidInstance(
            UriReferenceError::Invalid
        ))
    ));
}

#[test]
fn correlation_ids_encode_as_one_unambiguous_path_segment() {
    for (source, expected) in [
        ("abc/def", "abc%2Fdef"),
        ("abc?def", "abc%3Fdef"),
        ("abc#def", "abc%23def"),
        ("..", "%2E%2E"),
        ("%2F", "%252F"),
        ("foo\\bar", "foo%5Cbar"),
    ] {
        let Ok(id) = CorrelationId::new(source) else {
            panic!("test correlation ID must be valid");
        };
        assert_eq!(id.to_uri_path_segment(), expected);
    }
}
