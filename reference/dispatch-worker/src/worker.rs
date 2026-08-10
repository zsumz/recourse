//! In-memory durable-failure workflow over service-constructed diagnostics.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};

use dispatch_diagnostics::DispatchCatalog;
use dispatch_service::{DispatchFailure, DispatchService, JobIdGenerator};
use recourse::{catalog::Catalog, operation::OperationDiagnosticId};

use crate::{
    DispatchWorkerError, PublishedQueueHealth, RecordFailureOutcome, RecordedDispatchFailure,
};

/// Cloneable runtime-neutral worker with an in-memory durable record store.
#[derive(Debug, Clone, Default)]
pub struct DispatchWorker {
    records: Arc<Mutex<BTreeMap<OperationDiagnosticId, RecordedDispatchFailure>>>,
    queue_health: Arc<Mutex<Option<PublishedQueueHealth>>>,
}

impl DispatchWorker {
    /// Creates an empty reference worker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a typed failure for one accepted job, idempotently.
    pub fn record_failure<G: JobIdGenerator>(
        &self,
        catalog: &Catalog<DispatchCatalog>,
        service: &DispatchService<G>,
        failure: &DispatchFailure,
    ) -> Result<RecordFailureOutcome, DispatchWorkerError> {
        let diagnostic_id = failure
            .try_diagnostic_id()
            .map_err(DispatchWorkerError::Diagnostic)?;
        let mut records = self.lock_records()?;
        if let Some(record) = records.get(&diagnostic_id) {
            return replay(record, failure, diagnostic_id);
        }
        let failed = service
            .try_fail_job(catalog, failure)
            .map_err(DispatchWorkerError::Diagnostic)?;
        let body = failed
            .diagnostic()
            .try_encode_value()
            .map_err(DispatchWorkerError::Encode)?;
        let record = RecordedDispatchFailure::new(
            diagnostic_id.clone(),
            failed.job().clone(),
            failure.attempt(),
            failure.impact().clone(),
            body,
        );
        records.insert(diagnostic_id, record.clone());
        Ok(RecordFailureOutcome::Recorded(record))
    }

    /// Returns a previously recorded durable failure by occurrence identity.
    pub fn recorded_failure(
        &self,
        diagnostic_id: &OperationDiagnosticId,
    ) -> Result<Option<RecordedDispatchFailure>, DispatchWorkerError> {
        Ok(self.lock_records()?.get(diagnostic_id).cloned())
    }

    /// Publishes the finding the service reports for its current queue state.
    pub fn publish_queue_health<G: JobIdGenerator>(
        &self,
        catalog: &Catalog<DispatchCatalog>,
        service: &DispatchService<G>,
    ) -> Result<PublishedQueueHealth, DispatchWorkerError> {
        let finding = service
            .try_queue_finding(catalog)
            .map_err(DispatchWorkerError::Diagnostic)?;
        let body = finding
            .try_encode_value()
            .map_err(DispatchWorkerError::HealthEncode)?;
        let published = PublishedQueueHealth::new(service.admission().queue().clone(), body);
        *self.lock_queue_health()? = Some(published.clone());
        Ok(published)
    }

    /// Returns the worker's current published queue finding, when degraded.
    pub fn published_queue_health(
        &self,
    ) -> Result<Option<PublishedQueueHealth>, DispatchWorkerError> {
        Ok(self.lock_queue_health()?.clone())
    }

    fn lock_records(
        &self,
    ) -> Result<
        MutexGuard<'_, BTreeMap<OperationDiagnosticId, RecordedDispatchFailure>>,
        DispatchWorkerError,
    > {
        self.records
            .lock()
            .map_err(|_| DispatchWorkerError::RecordStorePoisoned)
    }

    fn lock_queue_health(
        &self,
    ) -> Result<MutexGuard<'_, Option<PublishedQueueHealth>>, DispatchWorkerError> {
        self.queue_health
            .lock()
            .map_err(|_| DispatchWorkerError::HealthStorePoisoned)
    }
}

fn replay(
    record: &RecordedDispatchFailure,
    failure: &DispatchFailure,
    diagnostic_id: OperationDiagnosticId,
) -> Result<RecordFailureOutcome, DispatchWorkerError> {
    if record.matches(failure.attempt(), failure.impact()) {
        return Ok(RecordFailureOutcome::Replayed(record.clone()));
    }
    Err(DispatchWorkerError::ConflictingReplay { diagnostic_id })
}
