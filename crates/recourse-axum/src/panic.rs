//! Private representation of a recovered pre-response panic payload.

use std::{
    any::Any,
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug)]
pub(crate) struct RecoveredPanic {
    message: String,
}

impl RecoveredPanic {
    pub(crate) fn from_payload(payload: &(dyn Any + Send)) -> Self {
        let message = payload
            .downcast_ref::<&'static str>()
            .map(|value| (*value).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_owned());
        Self { message }
    }
}

impl Display for RecoveredPanic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "request boundary panicked: {}", self.message)
    }
}

impl Error for RecoveredPanic {}
