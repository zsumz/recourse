//! Startup failures for catalog and Recourse layer construction.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use dispatch_worker::DispatchWorkerError;
use recourse::catalog::CatalogBuildError;
use recourse::health::{HealthFindingIdError, ObservationTimeError};
use recourse_axum::LayerBuildError;

/// Invalid static configuration that prevents the reference API from starting.
#[derive(Debug)]
pub enum ApiBuildError {
    /// Dispatch catalog declarations failed validation.
    Catalog(CatalogBuildError),
    /// Axum lifecycle configuration failed validation.
    Layer(LayerBuildError),
    /// Static health-finding identity failed validation.
    HealthFindingId(HealthFindingIdError),
    /// Initial health observation could not be represented canonically.
    ObservationTime(ObservationTimeError),
    /// Initial typed health publication failed.
    HealthPublication(DispatchWorkerError),
}

impl Display for ApiBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(formatter, "build Dispatch catalog: {error}"),
            Self::Layer(error) => write!(formatter, "build Recourse layer: {error}"),
            Self::HealthFindingId(error) => write!(formatter, "build health finding ID: {error}"),
            Self::ObservationTime(error) => write!(formatter, "build observation time: {error}"),
            Self::HealthPublication(error) => write!(formatter, "publish initial health: {error}"),
        }
    }
}

impl Error for ApiBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(error) => Some(error),
            Self::Layer(error) => Some(error),
            Self::HealthFindingId(error) => Some(error),
            Self::ObservationTime(error) => Some(error),
            Self::HealthPublication(error) => Some(error),
        }
    }
}

impl From<CatalogBuildError> for ApiBuildError {
    fn from(error: CatalogBuildError) -> Self {
        Self::Catalog(error)
    }
}

impl From<LayerBuildError> for ApiBuildError {
    fn from(error: LayerBuildError) -> Self {
        Self::Layer(error)
    }
}

impl From<HealthFindingIdError> for ApiBuildError {
    fn from(error: HealthFindingIdError) -> Self {
        Self::HealthFindingId(error)
    }
}

impl From<ObservationTimeError> for ApiBuildError {
    fn from(error: ObservationTimeError) -> Self {
        Self::ObservationTime(error)
    }
}

impl From<DispatchWorkerError> for ApiBuildError {
    fn from(error: DispatchWorkerError) -> Self {
        Self::HealthPublication(error)
    }
}
