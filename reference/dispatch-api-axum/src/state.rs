//! Application state shared by thin transport handlers.

use dispatch_service::DispatchService;
use dispatch_worker::DispatchWorker;

#[derive(Debug, Clone)]
pub(crate) struct ApiState {
    pub(crate) service: DispatchService,
    pub(crate) worker: DispatchWorker,
}
