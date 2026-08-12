//! Precompiled value validators owned by a runtime catalog.

use std::{collections::BTreeMap, sync::Arc};

use serde_json::Value;

use super::{CatalogDiagnostic, CatalogIssue, CodeNumber, schema};

#[derive(Debug)]
pub(crate) struct DiagnosticValidators {
    evidence: Arc<jsonschema::Validator>,
    impact: Option<Arc<jsonschema::Validator>>,
}

impl DiagnosticValidators {
    pub(crate) fn evidence(&self) -> Arc<jsonschema::Validator> {
        Arc::clone(&self.evidence)
    }

    pub(crate) fn impact(&self) -> Option<Arc<jsonschema::Validator>> {
        self.impact.as_ref().map(Arc::clone)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValueViolation {
    pub(crate) path: String,
    pub(crate) reason: String,
}

pub(crate) fn compile_all(
    diagnostics: &[CatalogDiagnostic],
    issues: &mut Vec<CatalogIssue>,
) -> BTreeMap<CodeNumber, DiagnosticValidators> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| compile_one(diagnostic, issues))
        .collect()
}

pub(crate) fn validate(
    validator: &jsonschema::Validator,
    value: &Value,
) -> Result<(), ValueViolation> {
    validator.validate(value).map_err(|error| ValueViolation {
        path: value_path(error.instance_path()),
        reason: error.to_string(),
    })
}

fn compile_one(
    diagnostic: &CatalogDiagnostic,
    issues: &mut Vec<CatalogIssue>,
) -> Option<(CodeNumber, DiagnosticValidators)> {
    let evidence = compile(diagnostic.evidence_schema()).map_err(|reason| {
        issues.push(CatalogIssue::UnsupportedEvidenceSchema {
            number: diagnostic.number(),
            path: "$".to_owned(),
            reason,
        });
    });
    let impact = diagnostic
        .impact_schema()
        .map(|schema| {
            compile(schema).map_err(|reason| {
                issues.push(CatalogIssue::UnsupportedImpactSchema {
                    number: diagnostic.number(),
                    path: "$".to_owned(),
                    reason,
                });
            })
        })
        .transpose();
    match (evidence, impact) {
        (Ok(evidence), Ok(impact)) => Some((
            diagnostic.number(),
            DiagnosticValidators { evidence, impact },
        )),
        (Err(()), _) | (_, Err(())) => None,
    }
}

fn compile(schema: &Value) -> Result<Arc<jsonschema::Validator>, String> {
    schema::build_validator(schema)
        .map(Arc::new)
        .map_err(|violation| violation.reason)
}

fn value_path(path: impl std::fmt::Display) -> String {
    let path = path.to_string();
    if path.is_empty() {
        "$".to_owned()
    } else {
        format!("${path}")
    }
}
