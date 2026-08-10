//! Surface registrations merged by permanent diagnostic marker and number.

use std::{any::TypeId, collections::BTreeMap};

use serde_json::Value;

use crate::{
    catalog::{
        CodeNumber,
        schema::{self, SchemaViolation},
    },
    diagnostic::DiagnosticType,
    health::HealthFindingType,
    http::{HttpPolicy, HttpProblemType},
    operation::OperationDiagnosticType,
};

#[derive(Debug)]
pub(super) struct Registration {
    pub(super) type_id: TypeId,
    pub(super) number: CodeNumber,
    pub(super) title: &'static str,
    pub(super) detail: &'static str,
    pub(super) suggestions: &'static [&'static str],
    pub(super) docs: &'static str,
    pub(super) evidence_schema: Result<Value, SchemaViolation>,
    pub(super) http: Option<HttpRegistration>,
    pub(super) operation: Option<OperationRegistration>,
    pub(super) health: bool,
}

impl Registration {
    pub(super) fn problem<D: HttpProblemType>() -> Self {
        let mut registration = Self::diagnostic::<D>();
        registration.http = Some(HttpRegistration {
            status: D::Policy::STATUS,
            policy: D::Policy::NAME,
            required_headers: D::Policy::REQUIRED_HEADERS,
        });
        registration
    }

    pub(super) fn operation<D: OperationDiagnosticType>() -> Self {
        let mut registration = Self::diagnostic::<D>();
        registration.operation = Some(OperationRegistration {
            impact_schema: schema::normalize::<D::Impact>(),
        });
        registration
    }

    pub(super) fn health<D: HealthFindingType>() -> Self {
        let mut registration = Self::diagnostic::<D>();
        registration.health = true;
        registration
    }

    pub(super) fn merge(&mut self, other: Self) {
        if self.http.is_none() {
            self.http = other.http;
        }
        if self.operation.is_none() {
            self.operation = other.operation;
        }
        self.health |= other.health;
    }

    fn diagnostic<D: DiagnosticType>() -> Self {
        Self {
            type_id: TypeId::of::<D>(),
            number: D::NUMBER,
            title: D::TITLE,
            detail: D::DETAIL,
            suggestions: D::SUGGESTIONS,
            docs: D::DOCS,
            evidence_schema: schema::normalize::<D::Evidence>(),
            http: None,
            operation: None,
            health: false,
        }
    }
}

#[derive(Debug)]
pub(super) struct HttpRegistration {
    pub(super) status: u16,
    pub(super) policy: &'static str,
    pub(super) required_headers: &'static [&'static str],
}

#[derive(Debug)]
pub(super) struct OperationRegistration {
    pub(super) impact_schema: Result<Value, SchemaViolation>,
}

pub(super) fn registered_problems(
    registrations: &BTreeMap<CodeNumber, Registration>,
) -> BTreeMap<TypeId, CodeNumber> {
    registered_where(registrations, |registration| registration.http.is_some())
}

pub(super) fn registered_operations(
    registrations: &BTreeMap<CodeNumber, Registration>,
) -> BTreeMap<TypeId, CodeNumber> {
    registered_where(registrations, |registration| {
        registration.operation.is_some()
    })
}

pub(super) fn registered_health(
    registrations: &BTreeMap<CodeNumber, Registration>,
) -> BTreeMap<TypeId, CodeNumber> {
    registered_where(registrations, |registration| registration.health)
}

fn registered_where(
    registrations: &BTreeMap<CodeNumber, Registration>,
    includes: fn(&Registration) -> bool,
) -> BTreeMap<TypeId, CodeNumber> {
    registrations
        .values()
        .filter(|registration| includes(registration))
        .map(|registration| (registration.type_id, registration.number))
        .collect()
}
