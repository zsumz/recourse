//! Non-serializable source errors and operator-only diagnostic context.

use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

/// One ordered operator-only key-value context entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateContext {
    key: String,
    value: String,
}

impl PrivateContext {
    /// Stable operator-facing context key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Private operator-facing value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Source error and private context that can never be Problem evidence.
pub struct PrivateReport {
    source: Box<dyn Error + Send + Sync>,
    context: Vec<PrivateContext>,
}

impl PrivateReport {
    /// Starts an operator-only report from a concrete source error.
    pub fn new<E>(source: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            source: Box::new(source),
            context: Vec::new(),
        }
    }

    /// Appends one ordered private context entry.
    #[must_use]
    pub fn context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push(PrivateContext {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    /// Root private source error.
    pub fn source_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.source.as_ref()
    }

    /// Ordered private context entries.
    pub fn contexts(&self) -> &[PrivateContext] {
        &self.context
    }
}

impl Debug for PrivateReport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateReport")
            .field("source", &self.source)
            .field("context", &self.context)
            .finish()
    }
}

impl Display for PrivateReport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.source, formatter)?;
        for entry in &self.context {
            write!(formatter, " [{}={}]", entry.key, entry.value)?;
        }
        Ok(())
    }
}

impl Error for PrivateReport {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
