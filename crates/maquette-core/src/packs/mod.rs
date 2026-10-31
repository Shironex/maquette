//! Export packs: a set of models made ready for another engine or a friend,
//! as one folder (and a zip next to it) of plain files.
//!
//! A pack starts from library entries (a selection or a collection) or a
//! project's accepted models. Each model runs headless in Blender on its own
//! ([`run`]): its real size and fix-ups as the project has them, then the
//! pack's own (a decimate to the kind's budget, textures capped, the origin
//! at the bottom centre), written as OBJ with its MTL and PNG, as GLB, or
//! both, and opened again in an empty scene to check what the other side
//! gets. A check that loses thin parts or misses a texture waits for a
//! person ([`PackDecision`]); the zip waits until every model passes or is
//! kept. Then [`finish`] writes the README, the manifest and the contact
//! sheet and zips the folder.
//!
//! ```text
//! <out>/village-props/
//!   README.txt  manifest.json  contact-sheet.png
//!   cast_iron_stove/cast_iron_stove.obj, .mtl, .png
//! <out>/village-props.zip
//! ```
//!
//! `<out>` is `<workspace>/exports` unless the person picks a folder; an
//! agent may only name one inside the workspace. Every pack has a record in
//! `<workspace>/exports/.packs/<id>.json`, so recent packs list and a pack
//! Blender stopped in goes on from the model it reached. Animation clips
//! that came from Mixamo are never written: Mixamo allows using them, not
//! sharing them as files ([`licence`]).

use serde::{Deserialize, Serialize};

use crate::assets::AssetKind;
use crate::error::{CoreError, CoreResult};

mod decide;
mod finish;
mod job;
pub mod licence;
mod model_run;
mod plan;
mod presets;
mod record;
mod sheet;

pub use decide::PackDecision;
pub use finish::{ZipChoice, finish};
pub use job::{PackEvent, Packs};
pub use plan::{KindRow, PackModel, PackPlan, SourceOf, plan};
pub use presets::{PackPreset, presets, save_preset};
pub use record::{
    PackItem, PackItemState, PackRecord, PackState, load as load_record, out_dir, recent,
};

/// The exports folder, under the workspace root.
pub const EXPORTS_DIR: &str = "exports";
/// Where the pack records live, under the exports folder.
pub const RECORDS_DIR: &str = ".packs";
/// The longest pack name.
pub const MAX_NAME: usize = 60;
/// The most models one pack takes.
pub const MAX_MODELS: usize = 500;
/// Texture edges a pack may cap to.
pub const TEXTURE_SIZES: [u32; 4] = [512, 1024, 2048, 4096];

/// The files each model becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PackFormat {
    /// OBJ with its MTL and PNG textures.
    Obj,
    /// Binary glTF with the textures inside.
    Glb,
    Both,
}

impl PackFormat {
    pub fn obj(self) -> bool {
        matches!(self, Self::Obj | Self::Both)
    }

    pub fn glb(self) -> bool {
        matches!(self, Self::Glb | Self::Both)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Obj => "OBJ",
            Self::Glb => "GLB",
            Self::Both => "OBJ and GLB",
        }
    }
}

/// What happens to the GLB files after Blender writes them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum PackCompression {
    /// Plain files any importer opens.
    #[default]
    None,
    /// The workspace's compress settings, as the pipeline's Compress step
    /// runs them (meshopt and KTX2): small, but only for engines that read
    /// both. The check opens the plain file before it is compressed.
    Web,
}

/// The triangle budget of one kind; `None` leaves that kind as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct KindBudget {
    pub kind: AssetKind,
    pub triangles: Option<u32>,
}

/// How a pack is made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackSettings {
    /// The folder and zip name: lowercase words joined by dashes.
    pub name: String,
    pub format: PackFormat,
    /// Scales each model to the real size its project gave it (metres).
    pub real_size: bool,
    /// Puts each model's origin at its bottom centre, on the floor.
    pub pivot_bottom: bool,
    /// Names each model's materials body, body_2 and so on.
    pub one_material: bool,
    /// The longest texture edge in pixels; 0 keeps every texture as it is.
    pub texture: u32,
    /// With OBJ, also writes a GLB with the bones and clips of a rigged
    /// model (OBJ cannot hold a rig).
    pub rigged_glb: bool,
    pub compression: PackCompression,
    pub budgets: Vec<KindBudget>,
    pub readme: bool,
    pub manifest: bool,
    pub contact_sheet: bool,
    /// Copies each model's concept and refs beside it.
    pub refs: bool,
    /// Writes `<name>.zip` next to the folder.
    pub zip: bool,
}

impl Default for PackSettings {
    fn default() -> Self {
        Self {
            name: "pack".to_string(),
            format: PackFormat::Obj,
            real_size: true,
            pivot_bottom: true,
            one_material: true,
            texture: 1024,
            rigged_glb: true,
            compression: PackCompression::None,
            budgets: vec![
                KindBudget {
                    kind: AssetKind::Prop,
                    triangles: Some(3_000),
                },
                KindBudget {
                    kind: AssetKind::Character,
                    triangles: None,
                },
                KindBudget {
                    kind: AssetKind::Scene,
                    triangles: Some(20_000),
                },
            ],
            readme: true,
            manifest: true,
            contact_sheet: true,
            refs: false,
            zip: true,
        }
    }
}

impl PackSettings {
    /// The budget of `kind`, when it has one.
    pub fn budget(&self, kind: AssetKind) -> Option<u32> {
        self.budgets
            .iter()
            .find(|b| b.kind == kind)
            .and_then(|b| b.triangles)
    }

    /// Checks the settings and gives back the name as a folder name.
    ///
    /// # Errors
    ///
    /// Refuses an empty or too long name, a texture size the pack does not
    /// offer and a budget under 100 triangles.
    pub fn checked(&self) -> CoreResult<Self> {
        if !self.name.chars().any(char::is_alphanumeric) {
            return Err(CoreError::Invalid("Give the pack a name.".to_string()));
        }
        let name = crate::project::slugify(self.name.trim());
        if name.chars().count() > MAX_NAME {
            return Err(CoreError::Invalid(format!(
                "Keep the pack name to {MAX_NAME} characters or fewer."
            )));
        }
        if self.texture != 0 && !TEXTURE_SIZES.contains(&self.texture) {
            return Err(CoreError::Invalid(
                "Pick a texture size of 512, 1024, 2048 or 4096 px, or keep them as they are."
                    .to_string(),
            ));
        }
        let low = crate::prepare::MIN_TRIANGLES;
        if let Some(bad) = self
            .budgets
            .iter()
            .find(|b| b.triangles.is_some_and(|t| t < low))
        {
            return Err(CoreError::Invalid(format!(
                "A {} budget of fewer than {low} triangles leaves little of a model.",
                kind_label(bad.kind).to_lowercase()
            )));
        }
        Ok(Self {
            name,
            ..self.clone()
        })
    }
}

/// What the pack is made of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "from", rename_all = "kebab-case")]
pub enum PackSource {
    /// Library entries, each at its newest version.
    Entries { entries: Vec<String> },
    /// Every entry of a library collection.
    Collection { collection: String },
    /// Every accepted model of a project, at its newest version.
    Project { project: String },
}

/// Everything a pack needs to start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PackRequest {
    pub source: PackSource,
    pub settings: PackSettings,
    /// The folder the pack folder and zip go in. `None` means
    /// `<workspace>/exports`.
    pub out: Option<String>,
    /// The preset it was made from, counted as used.
    pub preset: Option<String>,
}

/// "Props", "Characters", "Scenes".
pub fn kind_label(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Prop => "Props",
        AssetKind::Character => "Characters",
        AssetKind::Scene => "Scenes",
    }
}

#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod tests_blender;
#[cfg(test)]
mod tests_job;

/// Registers this module's IPC types (pack settings, plans, presets and
/// records) with the shell's TypeScript export; `maquette_core::types`
/// folds every module's in.
pub fn register(types: specta::Types) -> specta::Types {
    types
        .register::<PackFormat>()
        .register::<PackCompression>()
        .register::<KindBudget>()
        .register::<PackSettings>()
        .register::<PackSource>()
        .register::<PackRequest>()
        .register::<PackModel>()
        .register::<KindRow>()
        .register::<PackPlan>()
        .register::<PackPreset>()
        .register::<PackItemState>()
        .register::<PackItem>()
        .register::<PackState>()
        .register::<PackRecord>()
        .register::<PackDecision>()
        .register::<ZipChoice>()
}
