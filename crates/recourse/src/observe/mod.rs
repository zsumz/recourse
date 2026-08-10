//! Bounded metadata observation and separate private fault reporting ports.

mod context;
mod event;
mod port;

pub use context::{
    HttpEventContext, MAX_NORMALIZED_ROUTE_BYTES, NormalizedRoute, NormalizedRouteError,
};
pub use event::{EventSurface, FaultEvent, ProblemEvent};
pub use port::{FaultReporter, HttpObserver};

#[cfg(test)]
mod context_test;
#[cfg(test)]
mod event_test;
#[cfg(test)]
mod port_test;
