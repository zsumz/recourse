//! Public job model wire-shape tests.

use super::{CreateJobRequest, Destination, Job, JobId, JobState};

#[test]
fn accepted_job_has_a_stable_public_wire_shape() {
    let id = JobId::new("job_01K00000000000000000000000")
        .unwrap_or_else(|error| panic!("test job ID must be valid: {error}"));
    let destination = Destination::new("warehouse-west")
        .unwrap_or_else(|error| panic!("test destination must be valid: {error}"));
    let job = Job {
        id,
        destination,
        state: JobState::Accepted,
    };

    let wire = serde_json::to_string(&job)
        .unwrap_or_else(|error| panic!("test job must serialize: {error}"));
    assert_eq!(
        wire,
        r#"{"id":"job_01K00000000000000000000000","destination":"warehouse-west","state":"accepted"}"#
    );
}

#[test]
fn create_request_revalidates_destination_during_decoding() {
    assert!(serde_json::from_str::<CreateJobRequest>(r#"{"destination":"queue-east"}"#).is_ok());
    assert!(serde_json::from_str::<CreateJobRequest>(r#"{"destination":""}"#).is_err());
}
