//! Focused tests for canonical Dispatch job identifiers.

use std::str::FromStr;

use super::{JobId, JobIdError};

const VALID: &str = "job_01K00000000000000000000000";

#[test]
fn canonical_job_id_round_trips() {
    let id = JobId::from_str(VALID);

    assert_eq!(id.as_ref().map(JobId::as_str), Ok(VALID));
    let Some(id) = id.ok() else {
        return;
    };
    assert!(matches!(
        serde_json::to_string(&id),
        Ok(value) if value == format!("\"{VALID}\"")
    ));
}

#[test]
fn malformed_job_ids_are_rejected() {
    assert_eq!(JobId::new("01K"), Err(JobIdError::InvalidPrefix));
    assert_eq!(
        JobId::new("job_01K"),
        Err(JobIdError::InvalidLength { actual: 3 })
    );
    assert!(matches!(
        JobId::new("job_01I00000000000000000000000"),
        Err(JobIdError::InvalidCharacter { .. })
    ));
    assert!(matches!(
        JobId::new("job_81K00000000000000000000000"),
        Err(JobIdError::InvalidCharacter { index: 4, .. })
    ));
    assert!(serde_json::from_str::<JobId>("\"not-a-job\"").is_err());
}
