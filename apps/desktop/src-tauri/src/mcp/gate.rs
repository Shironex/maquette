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
    maquette_core::pipeline::is_ship_step(step)
}

/// Why a run waits before Ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipReason {
    /// An agent started the run and "Ask before shipping" is on.
    Agent,
    /// A game project: Ship writes into the game repo, so it always asks.
    Game,
}

pub struct ShipGate {
    pub hub: Arc<Hub>,
    pub reason: ShipReason,
}

impl ShipGate {
    /// The gate for a run an agent starts.
    pub fn for_agent(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            reason: ShipReason::Agent,
        }
    }
}

impl StepGate for ShipGate {
    fn applies(&self, step: &StepDef) -> bool {
        is_ship(step)
    }

    fn waiting_message(&self, step: &StepDef) -> String {
        match self.reason {
            ShipReason::Agent => format!(
                "{} waits for approval in Maquette, because an agent started this run.",
                step.name
            ),
            ShipReason::Game => format!(
                "{} waits for approval in Maquette, because it writes into the game repo.",
                step.name
            ),
        }
    }

    fn wait(&self, run: &RunRecord, step: &StepDef, cancelled: &AtomicBool) -> GateDecision {
        let id = self.hub.request_approval(run, step);
        match self.hub.wait_decision(&id, cancelled) {
            Some(true) => GateDecision::Allow,
            Some(false) => GateDecision::Deny(DENIED.to_string()),
            None => GateDecision::Deny("Cancelled.".to_string()),
        }
    }
}
