//! Deterministic serializable catalog snapshot.

mod parse;
mod parts;
mod surface;

use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Write},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Code, CodeNumber};
pub(crate) use parts::DiagnosticArtifactParts;
pub(crate) use surface::{DiagnosticSurfaces, HealthSurface, HttpSurface, OperationSurface};

/// Deterministic generated representation of one validated catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogArtifact {
    schema_version: u32,
    catalog: CatalogIdentity,
    diagnostics: Vec<CatalogDiagnostic>,
    problem_sets: BTreeMap<String, Vec<Code>>,
}

impl CatalogArtifact {
    pub(crate) fn new(
        identity: CatalogIdentity,
        diagnostics: Vec<CatalogDiagnostic>,
        problem_sets: BTreeMap<String, Vec<Code>>,
    ) -> Self {
        Self {
            schema_version: 1,
            catalog: identity,
            diagnostics,
            problem_sets,
        }
    }

    /// Artifact schema version.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Stable catalog name.
    pub fn name(&self) -> &str {
        &self.catalog.name
    }

    /// Stable catalog code prefix.
    pub fn prefix(&self) -> &str {
        &self.catalog.prefix
    }

    /// Absolute base used to derive diagnostic type URIs.
    pub fn type_base(&self) -> &str {
        &self.catalog.type_base
    }

    /// Diagnostics sorted by their positive numeric identity.
    pub fn diagnostics(&self) -> &[CatalogDiagnostic] {
        &self.diagnostics
    }

    /// HTTP diagnostic codes declared for each stable API operation ID.
    pub const fn problem_sets(&self) -> &BTreeMap<String, Vec<Code>> {
        &self.problem_sets
    }

    /// Parses and semantically validates a bounded catalog artifact.
    pub fn from_slice(body: &[u8]) -> Result<Self, ArtifactParseError> {
        parse::parse_artifact(body)
    }

    /// Writes canonical pretty JSON followed by one newline.
    pub fn write_pretty<W: Write>(&self, mut writer: W) -> Result<(), ArtifactWriteError> {
        serde_json::to_writer_pretty(&mut writer, self).map_err(ArtifactWriteError::Serialize)?;
        writer.write_all(b"\n").map_err(ArtifactWriteError::Write)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogIdentity {
    name: String,
    prefix: String,
    type_base: String,
}

impl CatalogIdentity {
    pub(crate) fn new(name: &str, prefix: &str, type_base: &str) -> Self {
        Self {
            name: name.to_owned(),
            prefix: prefix.to_owned(),
            type_base: type_base.to_owned(),
        }
    }
}

/// One validated semantic identity and its registered surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogDiagnostic {
    number: CodeNumber,
    code: Code,
    #[serde(rename = "type")]
    type_uri: String,
    title: String,
    detail: String,
    suggestions: Vec<String>,
    documentation_markdown: String,
    evidence_schema: Value,
    surfaces: DiagnosticSurfaces,
}

pub use parse::ArtifactParseError;

impl CatalogDiagnostic {
    /// Permanent numeric identity within the catalog.
    pub const fn number(&self) -> CodeNumber {
        self.number
    }

    /// Canonical compact code.
    pub fn code(&self) -> &Code {
        &self.code
    }

    /// Absolute semantic type URI.
    pub fn type_uri(&self) -> &str {
        &self.type_uri
    }

    /// Stable short title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Safe default caller-visible explanation.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Ordered caller guidance.
    pub fn suggestions(&self) -> &[String] {
        &self.suggestions
    }

    /// Authored Markdown documentation.
    pub fn documentation_markdown(&self) -> &str {
        &self.documentation_markdown
    }

    /// Reviewed public evidence schema.
    pub const fn evidence_schema(&self) -> &Value {
        &self.evidence_schema
    }

    /// Governed HTTP status when the Problem surface is registered.
    pub const fn http_status(&self) -> Option<u16> {
        self.surfaces.http_status()
    }

    /// Stable HTTP policy family name when the Problem surface is registered.
    pub fn http_policy(&self) -> Option<&str> {
        self.surfaces.http_policy()
    }

    /// Required response headers when the Problem surface is registered.
    pub fn required_headers(&self) -> Option<&[String]> {
        self.surfaces.required_headers()
    }

    /// Reviewed impact schema when the durable-operation surface is registered.
    pub const fn impact_schema(&self) -> Option<&Value> {
        self.surfaces.impact_schema()
    }

    /// Whether this diagnostic is registered as a health finding.
    pub const fn supports_health(&self) -> bool {
        self.surfaces.supports_health()
    }
}

/// Error writing a deterministic catalog artifact.
#[derive(Debug)]
pub enum ArtifactWriteError {
    /// JSON serialization failed.
    Serialize(serde_json::Error),
    /// The destination rejected the trailing newline.
    Write(io::Error),
}

impl Display for ArtifactWriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "serialize catalog artifact: {error}"),
            Self::Write(error) => write!(formatter, "write catalog artifact: {error}"),
        }
    }
}

impl Error for ArtifactWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
            Self::Write(error) => Some(error),
        }
    }
}
