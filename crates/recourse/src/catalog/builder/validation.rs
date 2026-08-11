//! Namespace, metadata, schema, duplicate, and status validation.

use std::collections::BTreeMap;

use http::StatusCode;

use super::registration::Registration;
use crate::{
    catalog::{
        CatalogIssue, CatalogSpec, Code, CodeNumber, artifact::CatalogIdentity, valid_type_base,
    },
    http::mandatory_headers,
};

pub(super) struct ValidatedNamespace {
    pub(super) identity: CatalogIdentity,
    pub(super) prefix: &'static str,
    pub(super) type_base: &'static str,
}

pub(super) fn validate_namespace<C: CatalogSpec>(
    issues: &mut Vec<CatalogIssue>,
) -> Option<ValidatedNamespace> {
    let name_valid = valid_name(C::NAME);
    let prefix_valid = Code::new(C::PREFIX, CodeNumber::new(1)).is_ok();
    let base_valid = valid_type_base(C::TYPE_BASE);
    if !name_valid {
        issues.push(CatalogIssue::InvalidName {
            value: C::NAME.into(),
        });
    }
    if !prefix_valid {
        issues.push(CatalogIssue::InvalidPrefix {
            value: C::PREFIX.into(),
        });
    }
    if !base_valid {
        issues.push(CatalogIssue::InvalidTypeBase {
            value: C::TYPE_BASE.into(),
        });
    }
    (name_valid && prefix_valid && base_valid).then(|| ValidatedNamespace {
        identity: CatalogIdentity::new(C::NAME, C::PREFIX, C::TYPE_BASE),
        prefix: C::PREFIX,
        type_base: C::TYPE_BASE,
    })
}

pub(super) fn validate_registrations(
    registrations: Vec<Registration>,
    issues: &mut Vec<CatalogIssue>,
) -> BTreeMap<CodeNumber, Registration> {
    let mut unique = BTreeMap::<CodeNumber, Registration>::new();
    for registration in registrations {
        match unique.get_mut(&registration.number) {
            Some(previous) if previous.type_id != registration.type_id => {
                issues.push(CatalogIssue::DuplicateNumber {
                    number: registration.number,
                });
            }
            Some(previous) => previous.merge(registration),
            None => {
                unique.insert(registration.number, registration);
            }
        }
    }
    for registration in unique.values() {
        validate_registration(registration, issues);
    }
    unique
}

fn validate_registration(registration: &Registration, issues: &mut Vec<CatalogIssue>) {
    for (field, value) in [
        ("title", registration.title),
        ("detail", registration.detail),
        ("documentation", registration.docs),
    ] {
        if value.trim().is_empty() {
            issues.push(CatalogIssue::InvalidMetadata {
                number: registration.number,
                field,
                reason: "must not be empty".into(),
            });
        }
    }
    for suggestion in registration.suggestions {
        if suggestion.trim().is_empty() {
            issues.push(CatalogIssue::InvalidMetadata {
                number: registration.number,
                field: "suggestions",
                reason: "entries must not be empty".into(),
            });
        }
    }
    if let Err(violation) = &registration.evidence_schema {
        issues.push(CatalogIssue::UnsupportedEvidenceSchema {
            number: registration.number,
            path: violation.path.clone(),
            reason: violation.reason.clone(),
        });
    }
    if let Some(operation) = &registration.operation
        && let Err(violation) = &operation.impact_schema
    {
        issues.push(CatalogIssue::UnsupportedImpactSchema {
            number: registration.number,
            path: violation.path.clone(),
            reason: violation.reason.clone(),
        });
    }
    if let Some(http) = &registration.http {
        let status = StatusCode::from_u16(http.status);
        if !status.is_ok_and(|value| value.is_client_error() || value.is_server_error()) {
            issues.push(CatalogIssue::InvalidHttpStatus {
                number: registration.number,
                status: http.status,
            });
        }
        for header in mandatory_headers(http.status) {
            if !http
                .required_headers
                .iter()
                .any(|declared| declared.eq_ignore_ascii_case(header))
            {
                issues.push(CatalogIssue::MissingMandatoryHeader {
                    number: registration.number,
                    status: http.status,
                    header,
                });
            }
        }
    }
}

fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let starts_with_letter = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
    starts_with_letter
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
