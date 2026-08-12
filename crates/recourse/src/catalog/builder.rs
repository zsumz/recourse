//! Explicit catalog registration and aggregated construction validation.

mod closure;
mod compile;
mod problem_set;
mod registration;
mod validation;

use std::{any::TypeId, collections::BTreeMap, fmt, marker::PhantomData};

use crate::{health::HealthFindingType, http::HttpProblemType, operation::OperationDiagnosticType};

use super::validator::compile_all;
use super::{
    CatalogArtifact, CatalogBuildError, CatalogDiagnostic, CatalogSpec, Code, CodeNumber,
    DiagnosticValidators, ProblemSet,
};
use compile::compile_diagnostics;
use problem_set::{compile_problem_sets, validate_problem_sets};
use registration::{Registration, registered_health, registered_operations, registered_problems};
use validation::{validate_namespace, validate_registrations};

/// Validated catalog assembled from explicit registrations.
pub struct Catalog<C: CatalogSpec> {
    artifact: CatalogArtifact,
    problems: BTreeMap<TypeId, CodeNumber>,
    operations: BTreeMap<TypeId, CodeNumber>,
    health: BTreeMap<TypeId, CodeNumber>,
    validators: BTreeMap<CodeNumber, DiagnosticValidators>,
    marker: PhantomData<fn() -> C>,
}

impl<C: CatalogSpec> Catalog<C> {
    /// Starts an empty explicit registration builder.
    pub fn builder() -> CatalogBuilder<C> {
        CatalogBuilder {
            registrations: Vec::new(),
            problem_sets: Vec::new(),
            marker: PhantomData,
        }
    }

    /// Returns a deterministic owned artifact snapshot.
    pub fn artifact(&self) -> CatalogArtifact {
        self.artifact.clone()
    }

    /// Looks up one registered definition by permanent code.
    pub fn diagnostic(&self, code: &Code) -> Option<&CatalogDiagnostic> {
        self.artifact
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.code() == code)
    }

    pub(crate) fn problem_definition<D>(&self) -> Option<&CatalogDiagnostic>
    where
        D: HttpProblemType<Catalog = C>,
    {
        let number = self.problems.get(&TypeId::of::<D>())?;
        self.artifact
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.number() == *number)
    }

    pub(crate) fn operation_definition<D>(&self) -> Option<&CatalogDiagnostic>
    where
        D: OperationDiagnosticType<Catalog = C>,
    {
        let number = self.operations.get(&TypeId::of::<D>())?;
        self.definition(*number)
    }

    pub(crate) fn health_definition<D>(&self) -> Option<&CatalogDiagnostic>
    where
        D: HealthFindingType<Catalog = C>,
    {
        let number = self.health.get(&TypeId::of::<D>())?;
        self.definition(*number)
    }

    pub(crate) fn validators(&self, number: CodeNumber) -> Option<&DiagnosticValidators> {
        self.validators.get(&number)
    }

    fn definition(&self, number: CodeNumber) -> Option<&CatalogDiagnostic> {
        self.artifact
            .diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.number() == number)
    }
}

impl<C: CatalogSpec> fmt::Debug for Catalog<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Catalog")
            .field("artifact", &self.artifact)
            .field("registered_problem_count", &self.problems.len())
            .field("registered_operation_count", &self.operations.len())
            .field("registered_health_count", &self.health.len())
            .field("compiled_validator_count", &self.validators.len())
            .finish()
    }
}

/// Consuming builder for deterministic explicit catalog registration.
pub struct CatalogBuilder<C: CatalogSpec> {
    registrations: Vec<Registration>,
    problem_sets: Vec<ProblemSet<C>>,
    marker: PhantomData<fn() -> C>,
}

impl<C: CatalogSpec> CatalogBuilder<C> {
    /// Registers an HTTP Problem diagnostic surface.
    ///
    /// A declaration cannot be registered into another catalog:
    ///
    /// ```compile_fail
    /// use recourse::{
    ///     catalog::{Catalog, CatalogSpec, CodeNumber},
    ///     diagnostic::{DiagnosticType, NoEvidence},
    ///     http::{Fixed, HttpProblemType},
    /// };
    ///
    /// enum First {}
    /// enum Second {}
    /// impl CatalogSpec for First {
    ///     const NAME: &'static str = "first";
    ///     const PREFIX: &'static str = "FST";
    ///     const TYPE_BASE: &'static str = "https://first.invalid/problems/";
    /// }
    /// impl CatalogSpec for Second {
    ///     const NAME: &'static str = "second";
    ///     const PREFIX: &'static str = "SND";
    ///     const TYPE_BASE: &'static str = "https://second.invalid/problems/";
    /// }
    /// enum SecondProblem {}
    /// impl DiagnosticType for SecondProblem {
    ///     type Catalog = Second;
    ///     type Evidence = NoEvidence;
    ///     const NUMBER: CodeNumber = CodeNumber::new(1);
    ///     const TITLE: &'static str = "Second problem";
    ///     const DETAIL: &'static str = "Owned by the second catalog.";
    ///     const SUGGESTIONS: &'static [&'static str] = &[];
    ///     const DOCS: &'static str = "Second catalog documentation.";
    /// }
    /// impl HttpProblemType for SecondProblem { type Policy = Fixed<400>; }
    ///
    /// let _ = Catalog::<First>::builder().problem::<SecondProblem>();
    /// ```
    #[must_use]
    pub fn problem<D>(mut self) -> Self
    where
        D: HttpProblemType<Catalog = C>,
    {
        self.registrations.push(Registration::problem::<D>());
        self
    }

    /// Registers a durable-operation diagnostic surface.
    #[must_use]
    pub fn operation<D>(mut self) -> Self
    where
        D: OperationDiagnosticType<Catalog = C>,
    {
        self.registrations.push(Registration::operation::<D>());
        self
    }

    /// Registers a current health-finding surface.
    #[must_use]
    pub fn health<D>(mut self) -> Self
    where
        D: HealthFindingType<Catalog = C>,
    {
        self.registrations.push(Registration::health::<D>());
        self
    }

    /// Registers the declared HTTP diagnostics for one API operation.
    #[must_use]
    pub fn problem_set(mut self, problem_set: ProblemSet<C>) -> Self {
        self.problem_sets.push(problem_set);
        self
    }

    /// Validates all declarations and returns every actionable issue together.
    pub fn build(self) -> Result<Catalog<C>, CatalogBuildError> {
        let mut issues = Vec::new();
        let namespace = validate_namespace::<C>(&mut issues);
        let registrations = validate_registrations(self.registrations, &mut issues);
        let problem_sets = validate_problem_sets(self.problem_sets, &registrations, &mut issues);
        let problems = registered_problems(&registrations);
        let operations = registered_operations(&registrations);
        let health = registered_health(&registrations);
        let diagnostics = namespace
            .as_ref()
            .map(|value| compile_diagnostics(value, registrations, &mut issues))
            .unwrap_or_default();
        let problem_sets = namespace
            .as_ref()
            .map(|value| compile_problem_sets(value.prefix, problem_sets))
            .unwrap_or_default();
        let validators = compile_all(&diagnostics, &mut issues);
        if !issues.is_empty() {
            return Err(CatalogBuildError::new(issues));
        }
        let Some(namespace) = namespace else {
            return Err(CatalogBuildError::new(Vec::new()));
        };
        let artifact = CatalogArtifact::new(namespace.identity, diagnostics, problem_sets);
        let artifact =
            closure::validate(&artifact).map_err(|issue| CatalogBuildError::new(vec![issue]))?;
        Ok(Catalog {
            artifact,
            problems,
            operations,
            health,
            validators,
            marker: PhantomData,
        })
    }
}

impl<C: CatalogSpec> fmt::Debug for CatalogBuilder<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CatalogBuilder")
            .field("registrations", &self.registrations)
            .field("problem_sets", &self.problem_sets)
            .finish()
    }
}
