//! Deterministic catalog descriptions for each registered envelope surface.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Optional governed surfaces attached to one semantic diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiagnosticSurfaces {
    #[serde(skip_serializing_if = "Option::is_none")]
    http: Option<HttpSurface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<OperationSurface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<HealthSurface>,
}

impl DiagnosticSurfaces {
    pub(crate) const fn new(
        http: Option<HttpSurface>,
        operation: Option<OperationSurface>,
        health: Option<HealthSurface>,
    ) -> Self {
        Self {
            http,
            operation,
            health,
        }
    }

    pub(crate) const fn http_status(&self) -> Option<u16> {
        match &self.http {
            Some(surface) => Some(surface.status),
            None => None,
        }
    }

    pub(crate) fn http_policy(&self) -> Option<&str> {
        self.http.as_ref().map(|surface| surface.policy.as_str())
    }

    pub(crate) fn required_headers(&self) -> Option<&[String]> {
        self.http
            .as_ref()
            .map(|surface| surface.required_headers.as_slice())
    }

    pub(crate) const fn impact_schema(&self) -> Option<&Value> {
        match &self.operation {
            Some(surface) => Some(&surface.impact_schema),
            None => None,
        }
    }

    pub(crate) const fn supports_health(&self) -> bool {
        self.health.is_some()
    }

    pub(crate) const fn supports_http(&self) -> bool {
        self.http.is_some()
    }

    pub(crate) const fn supports_operation(&self) -> bool {
        self.operation.is_some()
    }
}

/// Governed HTTP policy data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HttpSurface {
    status: u16,
    policy: String,
    required_headers: Vec<String>,
}

impl HttpSurface {
    pub(crate) fn new(status: u16, policy: &str, required_headers: &[&str]) -> Self {
        Self {
            status,
            policy: policy.to_owned(),
            required_headers: required_headers.iter().map(ToString::to_string).collect(),
        }
    }
}

/// Governed durable-operation impact shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OperationSurface {
    impact_schema: Value,
}

impl OperationSurface {
    pub(crate) const fn new(impact_schema: Value) -> Self {
        Self { impact_schema }
    }
}

/// Marker object for the health-finding surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HealthSurface {}

impl HealthSurface {
    pub(crate) const fn new() -> Self {
        Self {}
    }
}
