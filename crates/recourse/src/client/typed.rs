//! Typed evidence access for a matching known diagnostic marker.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    marker::PhantomData,
};

use serde::de::DeserializeOwned;
use serde_json::Value;

use http::StatusCode;

use crate::{
    catalog::{CatalogSpec, Code, CodeParseError},
    http::{HttpPolicy, HttpProblemType, mandatory_headers},
};

use super::ReceivedProblem;

/// Borrowed typed view over a matching received Problem.
#[derive(Debug, Clone, Copy)]
pub struct ReceivedTypedProblem<'a, D: HttpProblemType> {
    received: &'a ReceivedProblem,
    marker: PhantomData<fn() -> D>,
}

impl<'a, D: HttpProblemType> ReceivedTypedProblem<'a, D> {
    /// Decodes typed evidence while the parent retains its complete raw object.
    pub fn evidence(&self) -> Result<D::Evidence, TypedProblemError>
    where
        D::Evidence: DeserializeOwned,
    {
        let evidence = self
            .received
            .evidence()
            .ok_or(TypedProblemError::MissingEvidence)?;
        serde_json::from_value(Value::Object(evidence.clone())).map_err(TypedProblemError::Evidence)
    }

    /// Borrows the complete tolerant parent representation.
    pub const fn received(&self) -> &'a ReceivedProblem {
        self.received
    }

    /// Whether tolerant body parsing found no remaining protocol issue.
    pub fn is_conformant(&self) -> bool {
        self.received.protocol_issues().is_empty()
    }
}

impl ReceivedProblem {
    /// Returns a typed view only when code and type URI match the declaration.
    ///
    /// `Ok(None)` means the code belongs to another diagnostic, so a client
    /// can try each code it acts on without treating a mismatch as an error.
    /// A matching code paired with the wrong type URI is a protocol issue and
    /// is reported as one.
    ///
    /// ```
    /// use recourse::{
    ///     catalog::{CatalogSpec, CodeNumber},
    ///     client::{DecodeLimits, ReceivedProblem},
    ///     diagnostic::{DiagnosticType, PublicEvidence},
    ///     http::{Fixed, HttpProblemType},
    ///     dependencies::http::{HeaderMap, StatusCode},
    /// };
    /// use schemars::JsonSchema;
    /// use serde::{Deserialize, Serialize};
    ///
    /// # enum ServiceCatalog {}
    /// # impl CatalogSpec for ServiceCatalog {
    /// #     const NAME: &'static str = "example-service";
    /// #     const PREFIX: &'static str = "EXM";
    /// #     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
    /// # }
    /// #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    /// struct JobNotFoundEvidence {
    ///     job_id: String,
    /// }
    ///
    /// impl PublicEvidence for JobNotFoundEvidence {}
    ///
    /// # enum JobNotFound {}
    /// # impl DiagnosticType for JobNotFound {
    /// #     type Catalog = ServiceCatalog;
    /// #     type Evidence = JobNotFoundEvidence;
    /// #     const NUMBER: CodeNumber = CodeNumber::new(1003);
    /// #     const TITLE: &'static str = "Job not found";
    /// #     const DETAIL: &'static str = "No job exists for the supplied identifier.";
    /// #     const SUGGESTIONS: &'static [&'static str] = &["Check the job identifier."];
    /// #     const DOCS: &'static str = "Create a job before requesting its status.";
    /// # }
    /// # impl HttpProblemType for JobNotFound { type Policy = Fixed<404>; }
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let body = br#"{"type":"https://example.invalid/problems/EXM-1003",
    ///     "code":"EXM-1003","status":404,"evidence":{"job_id":"job_01K","added_later":7}}"#;
    /// let received = ReceivedProblem::from_slice(
    ///     StatusCode::NOT_FOUND,
    ///     &HeaderMap::new(),
    ///     body,
    ///     DecodeLimits::default(),
    /// )?;
    ///
    /// if let Some(problem) = received.try_as::<JobNotFound>()? {
    ///     let evidence: JobNotFoundEvidence = problem.evidence()?;
    ///     assert_eq!(evidence.job_id, "job_01K");
    ///     // The parent keeps every property this build does not know about.
    ///     assert!(problem.received().raw().contains_key("evidence"));
    /// }
    /// # Ok(())
    /// # }
    /// # assert!(example().is_ok());
    /// ```
    pub fn try_as<D>(&self) -> Result<Option<ReceivedTypedProblem<'_, D>>, TypedProblemError>
    where
        D: HttpProblemType,
    {
        let expected_code = Code::new(D::Catalog::PREFIX, D::NUMBER)
            .map_err(TypedProblemError::InvalidDeclaration)?;
        if self.code() != Some(&expected_code) {
            return Ok(None);
        }
        let expected_type = format!("{}{expected_code}", D::Catalog::TYPE_BASE);
        match self.type_uri() {
            Some(received) if received == expected_type => {}
            Some(received) => Err(TypedProblemError::TypeMismatch {
                expected: expected_type,
                received: received.to_owned(),
            })?,
            None => return Err(TypedProblemError::MissingType),
        }
        let expected_status = StatusCode::from_u16(D::Policy::STATUS)
            .map_err(|_| TypedProblemError::InvalidStatusDeclaration(D::Policy::STATUS))?;
        if self.transport_status() != expected_status {
            return Err(TypedProblemError::StatusMismatch {
                expected: expected_status,
                received: self.transport_status(),
            });
        }
        for header in mandatory_headers(D::Policy::STATUS)
            .iter()
            .copied()
            .chain(D::Policy::REQUIRED_HEADERS.iter().copied())
        {
            if !self.headers().contains_key(header) {
                return Err(TypedProblemError::MissingRequiredHeader { header });
            }
        }
        Ok(Some(ReceivedTypedProblem {
            received: self,
            marker: PhantomData,
        }))
    }
}

/// Typed known-code verification or evidence-decoding failure.
#[derive(Debug)]
pub enum TypedProblemError {
    /// Diagnostic's catalog declaration cannot produce a canonical code.
    InvalidDeclaration(CodeParseError),
    /// Diagnostic declaration contains an invalid HTTP status.
    InvalidStatusDeclaration(u16),
    /// Matching code omitted its required type identity.
    MissingType,
    /// Matching code was paired with another type URI.
    TypeMismatch {
        /// Type URI derived from the local declaration.
        expected: String,
        /// String-valued received type member.
        received: String,
    },
    /// Matching diagnostic arrived under the wrong transport status.
    StatusMismatch {
        /// Status fixed by the declaration.
        expected: StatusCode,
        /// Actual transport status.
        received: StatusCode,
    },
    /// Matching diagnostic omitted a declaration-required header.
    MissingRequiredHeader {
        /// Missing canonical header name.
        header: &'static str,
    },
    /// Matching Problem did not supply object-valued evidence.
    MissingEvidence,
    /// Evidence object did not decode into the declared public type.
    Evidence(serde_json::Error),
}

impl Display for TypedProblemError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeclaration(error) => {
                write!(formatter, "invalid local declaration: {error}")
            }
            Self::InvalidStatusDeclaration(status) => {
                write!(formatter, "invalid local HTTP status declaration {status}")
            }
            Self::MissingType => formatter.write_str("matching diagnostic code omitted its type"),
            Self::TypeMismatch { expected, received } => write!(
                formatter,
                "diagnostic type mismatch: expected {expected}, received {received}"
            ),
            Self::StatusMismatch { expected, received } => write!(
                formatter,
                "diagnostic status mismatch: expected {expected}, received {received}"
            ),
            Self::MissingRequiredHeader { header } => {
                write!(
                    formatter,
                    "diagnostic response omitted required header {header}"
                )
            }
            Self::MissingEvidence => {
                formatter.write_str("matching diagnostic omitted object evidence")
            }
            Self::Evidence(error) => write!(formatter, "decode typed evidence: {error}"),
        }
    }
}

impl Error for TypedProblemError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDeclaration(error) => Some(error),
            Self::Evidence(error) => Some(error),
            Self::InvalidStatusDeclaration(_)
            | Self::MissingType
            | Self::TypeMismatch { .. }
            | Self::StatusMismatch { .. }
            | Self::MissingRequiredHeader { .. }
            | Self::MissingEvidence => None,
        }
    }
}
