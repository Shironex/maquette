//! "Ask before shipping": a run an agent starts stops before its Ship step
//! until someone approves it in the app.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use maquette_core::pipeline::StepDef;
use maquette_core::pipeline::history::RunRecord;
use maquette_core::pipeline::runner::{GateDecision, StepGate};

use super::hub::Hub;

pub const DENIED: &str = "Shipping was denied in Maquette.";

/// The step that copies the build to the site.
pub fn is_ship(step: &StepDef) -> bool {
    step.id.trim().eq_ignore_ascii_case("ship") || step.name.trim().eq_ignore_ascii_case("ship")
}

pub struct ShipGate(pub Arc<Hub>);

impl StepGate for ShipGate {
    fn applies(&self, step: &StepDef) -> bool {
        is_ship(step)
    }

    fn waiting_message(&self, step: &StepDef) -> String {
        format!(
            "{} waits for approval in Maquette, because an agent started this run.",
            step.name
        )
    }

    fn wait(&self, run: &RunRecord, step: &StepDef, cancelled: &AtomicBool) -> GateDecision {
        let id = self.0.request_approval(run, step);
        match self.0.wait_decision(&id, cancelled) {
            Some(true) => GateDecision::Allow,
            Some(false) => GateDecision::Deny(DENIED.to_string()),
            None => GateDecision::Deny("Cancelled.".to_string()),
        }
    }
}
