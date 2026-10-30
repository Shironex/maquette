//! The springs a person confirmed for an avatar: which hanging parts the
//! finder proposed get joint chains, how many joints, and how they swing.
//! They live in the avatar spec as `springs`; a spec without them (and
//! every spec older versions wrote) has none.
//!
//! ```json
//! "springs": [
//!   { "id": "hair-1", "kind": "hair", "rootBone": "head",
//!     "line": [[-0.16, 1.51, -0.2], [-0.3, 0.6, -0.28]], "joints": 6,
//!     "preset": "soft", "dragForce": 0.3 }
//! ]
//! ```
//!
//! `id`, `kind`, `rootBone` and `line` come from the candidate; the line
//! finds the same part again when the chains are built. `joints` is picked
//! by length when left out, `preset` by kind, and `stiffness`,
//! `gravityPower`, `dragForce` and `hitRadius` override the preset's.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use maquette_glb::vrm::{SpringKind, SpringPreset, SpringSettings, is_human_bone};

/// The most joints a chain may have.
pub const SPRING_JOINTS_MAX: u32 = 16;
/// The most points a spring's line may hold.
pub const SPRING_LINE_MAX: usize = 64;
const ID_MAX: usize = 40;

/// One confirmed spring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SpringSpec {
    /// The candidate's id, such as `hair-1`.
    pub id: String,
    pub kind: SpringKind,
    /// The VRM humanoid bone the chain hangs from.
    pub root_bone: String,
    /// The candidate's centre line, glTF metres, from where it hangs.
    pub line: Vec<[f64; 3]>,
    /// Joints along the chain; by its length when `None`.
    #[serde(default)]
    pub joints: Option<u32>,
    /// By kind when `None`.
    #[serde(default)]
    pub preset: Option<SpringPreset>,
    #[serde(default)]
    pub stiffness: Option<f64>,
    #[serde(default)]
    pub gravity_power: Option<f64>,
    #[serde(default)]
    pub drag_force: Option<f64>,
    #[serde(default)]
    pub hit_radius: Option<f64>,
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Invalid(message.into())
}

impl SpringSpec {
    /// The preset, or the kind's.
    pub fn preset(&self) -> SpringPreset {
        self.preset.unwrap_or_else(|| self.kind.preset())
    }

    /// How the chain swings: the preset's settings with the overrides on top.
    pub fn settings(&self) -> SpringSettings {
        let base = self.preset().settings();
        SpringSettings {
            stiffness: self.stiffness.unwrap_or(base.stiffness),
            gravity_power: self.gravity_power.unwrap_or(base.gravity_power),
            drag_force: self.drag_force.unwrap_or(base.drag_force),
            hit_radius: self.hit_radius.unwrap_or(base.hit_radius),
            ..base
        }
    }

    /// Checks the id, the root bone, the line, the joint count and the
    /// overrides.
    ///
    /// # Errors
    ///
    /// The first problem, as a sentence for the person or agent.
    pub fn validate(&self) -> CoreResult<()> {
        let id_ok = !self.id.is_empty()
            && self.id.len() <= ID_MAX
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !id_ok {
            return Err(invalid(format!(
                "A spring id is 1 to {ID_MAX} lowercase letters, digits or dashes, such as hair-1."
            )));
        }
        let id = &self.id;
        if !is_human_bone(&self.root_bone) {
            return Err(invalid(format!(
                "Spring {id}: {} is not a VRM humanoid bone.",
                self.root_bone
            )));
        }
        let finite = self.line.iter().flatten().all(|v| v.is_finite());
        if self.line.len() < 2 || self.line.len() > SPRING_LINE_MAX || !finite {
            return Err(invalid(format!(
                "Spring {id}: its line needs 2 to {SPRING_LINE_MAX} points of finite numbers."
            )));
        }
        if let Some(n) = self.joints
            && !(1..=SPRING_JOINTS_MAX).contains(&n)
        {
            return Err(invalid(format!(
                "Spring {id}: joints is 1 to {SPRING_JOINTS_MAX}."
            )));
        }
        let ranges = [
            ("stiffness", self.stiffness, 4.0),
            ("gravityPower", self.gravity_power, 2.0),
            ("dragForce", self.drag_force, 1.0),
            ("hitRadius", self.hit_radius, 0.5),
        ];
        for (name, value, top) in ranges {
            if let Some(v) = value
                && !(v.is_finite() && (0.0..=top).contains(&v))
            {
                return Err(invalid(format!("Spring {id}: {name} is 0 to {top}.")));
            }
        }
        Ok(())
    }
}

/// Checks every spring and that no two share an id.
///
/// # Errors
///
/// The first problem, as a sentence for the person or agent.
pub fn validate_springs(springs: &[SpringSpec]) -> CoreResult<()> {
    for (i, spring) in springs.iter().enumerate() {
        spring.validate()?;
        if springs[..i].iter().any(|s| s.id == spring.id) {
            return Err(invalid(format!("Two springs are named {}.", spring.id)));
        }
    }
    Ok(())
}
