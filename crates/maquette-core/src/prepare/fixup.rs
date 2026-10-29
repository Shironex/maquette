//! The fix-ups: one click repairs replayed on every version of an asset, in
//! a fixed order whatever order they were added in.

use serde::{Deserialize, Serialize};

use super::trim_number;

/// The kinds of fix-up, in the order they are applied.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum FixupKind {
    Rotate,
    ApplyTransforms,
    RemoveLoose,
    Merge,
    SplitParts,
    Decimate,
    ShrinkTextures,
    OriginBottom,
}

impl FixupKind {
    pub const ALL: [FixupKind; 8] = [
        FixupKind::Rotate,
        FixupKind::ApplyTransforms,
        FixupKind::RemoveLoose,
        FixupKind::Merge,
        FixupKind::SplitParts,
        FixupKind::Decimate,
        FixupKind::ShrinkTextures,
        FixupKind::OriginBottom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FixupKind::Rotate => "Rotate",
            FixupKind::ApplyTransforms => "Apply transforms",
            FixupKind::RemoveLoose => "Remove loose parts",
            FixupKind::Merge => "Merge objects",
            FixupKind::SplitParts => "Split into parts",
            FixupKind::Decimate => "Decimate",
            FixupKind::ShrinkTextures => "Shrink textures",
            FixupKind::OriginBottom => "Origin to bottom centre",
        }
    }
}

/// What a split cuts the model along.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SplitBy {
    /// Each loose piece (faces that touch) is a part.
    #[default]
    Loose,
    /// Each material is a part.
    Material,
}

impl SplitBy {
    pub fn label(self) -> &'static str {
        match self {
            SplitBy::Loose => "by loose pieces",
            SplitBy::Material => "by material",
        }
    }
}

/// One repair. Each kind appears at most once per asset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Fixup {
    /// Turns the model about its centre by these angles in degrees, in the
    /// app's axes (y up): about x, then y, then z. The turn is baked into
    /// the meshes (rigged meshes keep it on their rig), so a model that came
    /// out of a provider leaning stands up straight without a new send.
    Rotate { x: f32, y: f32, z: f32 },
    /// Bakes object transforms into the meshes (rigged meshes keep theirs).
    ApplyTransforms,
    /// Drops loose parts whose size is at most `minShare` of the model's
    /// diagonal, 0 to 0.5.
    RemoveLoose {
        #[serde(rename = "minShare")]
        min_share: f32,
    },
    /// Joins the meshes into one, or one per material.
    Merge {
        #[serde(rename = "byMaterial")]
        by_material: bool,
    },
    /// Cuts each unrigged mesh into named parts (`part_1`, `part_2`, top
    /// first), by loose pieces or by material. A piece whose size is at most
    /// `mergeUnder` of the model's diagonal (0 to 0.2) joins its nearest
    /// bigger neighbour. The texture and UVs stay as they are.
    SplitParts {
        #[serde(default)]
        by: SplitBy,
        #[serde(rename = "mergeUnder")]
        merge_under: f32,
    },
    /// Collapses the model down to about this many triangles.
    Decimate { triangles: u32 },
    /// Scales every image down to this many pixels on its longest edge.
    ShrinkTextures {
        #[serde(rename = "maxSize")]
        max_size: u32,
    },
    /// Moves the model so its bottom centre is the origin, on the floor.
    OriginBottom,
}

impl Fixup {
    pub fn kind(&self) -> FixupKind {
        match self {
            Fixup::Rotate { .. } => FixupKind::Rotate,
            Fixup::ApplyTransforms => FixupKind::ApplyTransforms,
            Fixup::RemoveLoose { .. } => FixupKind::RemoveLoose,
            Fixup::Merge { .. } => FixupKind::Merge,
            Fixup::SplitParts { .. } => FixupKind::SplitParts,
            Fixup::Decimate { .. } => FixupKind::Decimate,
            Fixup::ShrinkTextures { .. } => FixupKind::ShrinkTextures,
            Fixup::OriginBottom => FixupKind::OriginBottom,
        }
    }

    /// "Decimate to 60k", "Shrink textures to 2048 px", "Rotate x 24.5°".
    pub fn describe(&self) -> String {
        match self {
            Fixup::Rotate { x, y, z } => {
                let turns: Vec<String> = [("x", x), ("y", y), ("z", z)]
                    .into_iter()
                    .filter(|(_, deg)| **deg != 0.0)
                    .map(|(axis, deg)| format!("{axis} {}°", trim_number(*deg)))
                    .collect();
                if turns.is_empty() {
                    "Rotate (no turn)".to_string()
                } else {
                    format!("Rotate {}", turns.join(", "))
                }
            }
            Fixup::Decimate { triangles } => {
                format!(
                    "Decimate to {}",
                    crate::inbox::checks::short_count(*triangles)
                )
            }
            Fixup::ShrinkTextures { max_size } => format!("Shrink textures to {max_size} px"),
            Fixup::RemoveLoose { min_share } => format!(
                "Remove loose parts under {}%",
                trim_number(min_share * 100.0)
            ),
            Fixup::Merge { by_material: true } => "Merge objects by material".to_string(),
            Fixup::SplitParts {
                by: SplitBy::Material,
                ..
            } => "Split into parts by material".to_string(),
            other => other.kind().label().to_string(),
        }
    }
}
