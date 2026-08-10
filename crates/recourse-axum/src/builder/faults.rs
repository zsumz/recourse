//! Distinct tracking of the builder's stated fault-reporting decision.

use std::sync::Arc;

use recourse::{
    fault::PrivateReport,
    observe::{FaultEvent, FaultReporter},
};

use super::LayerBuildError;

/// Fault-reporting decisions a builder has stated so far.
///
/// Naming a reporter and naming the discard opt-out are tracked as separate
/// decisions rather than one slot, because stating both is a contradiction the
/// builder must report instead of resolving silently.
pub(super) enum FaultChoice {
    /// Neither decision has been stated.
    Unstated,
    /// A private fault-reporting port was named.
    Reporter(Arc<dyn FaultReporter>),
    /// Private reports were deliberately discarded.
    Discard,
    /// Both a reporter and the discard opt-out were named.
    Contradictory,
}

impl FaultChoice {
    /// Names a reporter, replacing any reporter stated earlier.
    pub(super) fn with_reporter(self, reporter: Arc<dyn FaultReporter>) -> Self {
        match self {
            // Re-stating a reporter refines one decision rather than reversing
            // it, so the most recently named reporter wins.
            Self::Unstated | Self::Reporter(_) => Self::Reporter(reporter),
            Self::Discard | Self::Contradictory => Self::Contradictory,
        }
    }

    /// Names the deliberate discard, which repeats without changing anything.
    pub(super) fn with_discard(self) -> Self {
        match self {
            Self::Unstated | Self::Discard => Self::Discard,
            Self::Reporter(_) | Self::Contradictory => Self::Contradictory,
        }
    }

    /// Resolves one stated decision into the port the layer will use.
    pub(super) fn into_reporter(self) -> Result<Arc<dyn FaultReporter>, LayerBuildError> {
        match self {
            Self::Unstated => Err(LayerBuildError::MissingFaultReporter),
            Self::Contradictory => Err(LayerBuildError::ContradictoryFaultReporting),
            Self::Reporter(reporter) => Ok(reporter),
            Self::Discard => Ok(Arc::new(DiscardedFaults)),
        }
    }

    /// Names the stated decision for builder diagnostics.
    pub(super) const fn stated(&self) -> &'static str {
        match self {
            Self::Unstated => "unstated",
            Self::Reporter(_) => "fault_reporter",
            Self::Discard => "discard_faults",
            Self::Contradictory => "fault_reporter and discard_faults",
        }
    }
}

/// Named discard chosen through `RecourseLayerBuilder::discard_faults`, so no
/// configuration path drops private reports without saying so.
#[derive(Debug)]
struct DiscardedFaults;

impl FaultReporter for DiscardedFaults {
    fn report_fault(&self, _event: &FaultEvent, _report: &PrivateReport) {}
}
