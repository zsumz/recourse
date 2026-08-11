//! Bounded machine-readable validation evidence and source locations.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::diagnostic::{PublicEvidence, PublicText};

use super::{HeaderName, JsonPointer, ParameterName};

/// Hard protocol ceiling and default maximum validation violation count.
pub const DEFAULT_MAX_VIOLATIONS: usize = 100;

/// Stable machine classification for one rejected input value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViolationReason {
    /// A required value is absent.
    Required,
    /// A value has the wrong public format.
    InvalidFormat,
    /// A numeric, textual, or collection value exceeds its accepted range.
    OutOfRange,
    /// A syntactically valid value is not permitted in this context.
    NotAllowed,
    /// Two individually valid inputs conflict.
    Conflict,
}

/// Public request location that owns one validation violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSource {
    /// JSON request body location.
    Body {
        /// RFC 6901 pointer into the request document.
        pointer: JsonPointer,
    },
    /// Query-string parameter location.
    Query {
        /// Public parameter name; its value is never included automatically.
        parameter: ParameterName,
    },
    /// HTTP request header location.
    Header {
        /// Canonical field name; its value is never included.
        name: HeaderName,
    },
    /// Route path parameter location.
    Path {
        /// Public parameter name.
        parameter: ParameterName,
    },
}

/// One caller-correctable validation violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Violation {
    /// Stable machine-readable reason.
    pub reason: ViolationReason,
    /// Bounded caller-visible explanation.
    pub detail: PublicText,
    /// Structured request location.
    pub source: ViolationSource,
}

/// Nonempty bounded set of validation violations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationEvidence {
    violations: Vec<Violation>,
}

impl ValidationEvidence {
    /// Validates against [`DEFAULT_MAX_VIOLATIONS`].
    pub fn new(violations: Vec<Violation>) -> Result<Self, ValidationEvidenceError> {
        Self::with_max_violations(violations, DEFAULT_MAX_VIOLATIONS)
    }

    /// Validates against a caller-selected limit below the protocol ceiling.
    pub fn with_max_violations(
        violations: Vec<Violation>,
        max_violations: usize,
    ) -> Result<Self, ValidationEvidenceError> {
        if max_violations == 0 {
            return Err(ValidationEvidenceError::ZeroLimit);
        }
        if max_violations > DEFAULT_MAX_VIOLATIONS {
            return Err(ValidationEvidenceError::LimitTooLarge { max_violations });
        }
        if violations.is_empty() {
            return Err(ValidationEvidenceError::Empty);
        }
        if violations.len() > max_violations {
            return Err(ValidationEvidenceError::TooMany {
                actual: violations.len(),
                max_violations,
            });
        }
        Ok(Self { violations })
    }

    /// Borrows violations in deterministic caller-selected order.
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }
}

impl PublicEvidence for ValidationEvidence {}

impl<'de> Deserialize<'de> for ValidationEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireEvidence {
            violations: Vec<Violation>,
        }

        let wire = WireEvidence::deserialize(deserializer)?;
        Self::new(wire.violations).map_err(D::Error::custom)
    }
}

impl JsonSchema for ValidationEvidence {
    fn schema_name() -> Cow<'static, str> {
        "ValidationEvidence".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "violations": {
                    "type": "array",
                    "items": generator.subschema_for::<Violation>(),
                    "minItems": 1,
                    "maxItems": DEFAULT_MAX_VIOLATIONS
                }
            },
            "required": ["violations"]
        })
    }
}

/// Reason a validation evidence collection violates its protocol bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationEvidenceError {
    /// Configured maximum is zero.
    ZeroLimit,
    /// Configured maximum exceeds the protocol hard ceiling.
    LimitTooLarge {
        /// Rejected configured maximum.
        max_violations: usize,
    },
    /// Validation evidence contains no violations.
    Empty,
    /// Validation evidence contains more violations than configured.
    TooMany {
        /// Actual number of violations.
        actual: usize,
        /// Configured maximum number of violations.
        max_violations: usize,
    },
}

impl Display for ValidationEvidenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit => formatter.write_str("maximum violation count must be positive"),
            Self::LimitTooLarge { max_violations } => write!(
                formatter,
                "maximum violation count {max_violations} exceeds protocol ceiling {DEFAULT_MAX_VIOLATIONS}"
            ),
            Self::Empty => formatter.write_str("validation evidence must contain a violation"),
            Self::TooMany {
                actual,
                max_violations,
            } => write!(
                formatter,
                "validation evidence has {actual} violations; maximum is {max_violations}"
            ),
        }
    }
}

impl Error for ValidationEvidenceError {}
