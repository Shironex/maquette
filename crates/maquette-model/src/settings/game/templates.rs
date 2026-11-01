//! Game templates: a short built-in list of export settings for the usual
//! kinds of game asset. A template only fills settings a person can edit by
//! hand: the model format, the texture format, the texture size cap, whether
//! normal and metal-roughness maps stay, and budget rows per folder. It
//! brings no assets and no game source, moves no file, and the project does
//! not remember it.

use serde::{Deserialize, Serialize};

use super::{FolderBudget, GameExport, GameFormat, GameTextures, default_budgets};

/// One of the built-in templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GameTemplateId {
    PropSet,
    Character,
    LevelKit,
    LevelKitObj,
    WebGame,
    Godot,
    Unity,
    Unreal,
    Mobile,
    VrStandalone,
    GameJam,
}

/// Where the picker lists a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GameTemplateGroup {
    /// The shape of the models: props, a character, a level kit.
    Shape,
    /// A game engine.
    Engine,
    /// A device or a kind of project with its own limits.
    Target,
}

/// What a template sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameTemplate {
    pub id: GameTemplateId,
    pub name: String,
    /// What it is for, one line.
    pub detail: String,
    /// Why it has these values.
    pub why: String,
    pub group: GameTemplateGroup,
    pub format: GameFormat,
    /// The texture format it sets, none to leave the game's as it is.
    pub textures: Option<GameTextures>,
    /// The longest texture edge it sets, none to leave the game's as it is.
    pub texture_cap: Option<u32>,
    /// Whether it leaves normal and metal-roughness maps out, none to leave
    /// the game's as it is.
    pub drop_maps: Option<bool>,
    /// The folders it lays out, with the budget of each.
    pub budgets: Vec<FolderBudget>,
}

impl GameTemplate {
    /// `current` with what this template sets laid over it.
    pub fn export_over(&self, current: GameExport) -> GameExport {
        GameExport {
            textures: self.textures.unwrap_or(current.textures),
            texture_cap: self.texture_cap.unwrap_or(current.texture_cap),
            drop_maps: self.drop_maps.unwrap_or(current.drop_maps),
        }
    }
}

/// A budget row.
fn row(folder: &str, triangles: u32, texture: u32) -> FolderBudget {
    FolderBudget {
        folder: folder.to_string(),
        triangles,
        texture,
    }
}

/// The budget a new game gives the folder of that name, so a template and
/// a new game never disagree about `props` or `characters`.
fn known(folder: &str) -> Vec<FolderBudget> {
    default_budgets(&[folder.to_string()])
}

/// A modular piece is drawn many times in one view, so it gets far fewer
/// triangles than the 12k of a prop that stands alone.
fn kit() -> Vec<FolderBudget> {
    vec![FolderBudget {
        folder: "kit".to_string(),
        triangles: 5_000,
        texture: 1024,
    }]
}

/// What the engine and device templates set beyond the format.
struct Export {
    textures: GameTextures,
    cap: u32,
    drop_maps: bool,
}

const JUDGMENT: &str = "The per model numbers are judgment, not published limits.";

/// The templates, in the order the app lists them.
pub fn game_templates() -> Vec<GameTemplate> {
    use GameTemplateGroup::{Engine, Shape, Target};
    let shape = |id, name: &str, detail: &str, why: &str, format, budgets| GameTemplate {
        id,
        name: name.to_string(),
        detail: detail.to_string(),
        why: why.to_string(),
        group: Shape,
        format,
        textures: None,
        texture_cap: None,
        drop_maps: None,
        budgets,
    };
    let tuned =
        |id, group, name: &str, detail: &str, why: String, export: Export, budgets| GameTemplate {
            id,
            name: name.to_string(),
            detail: detail.to_string(),
            why,
            group,
            format: GameFormat::Glb,
            textures: Some(export.textures),
            texture_cap: Some(export.cap),
            drop_maps: Some(export.drop_maps),
            budgets,
        };
    let png = |cap| Export {
        textures: GameTextures::Png,
        cap,
        drop_maps: false,
    };
    let jpg = |cap, drop_maps| Export {
        textures: GameTextures::Jpg,
        cap,
        drop_maps,
    };
    vec![
        shape(
            GameTemplateId::PropSet,
            "Prop set",
            "One file per prop, no scene",
            "A .glb holds a prop and its textures in one file. The budget is what a new game gives props/.",
            GameFormat::Glb,
            known("props"),
        ),
        shape(
            GameTemplateId::Character,
            "Character",
            "One rigged model with its clips",
            "Rigs and clips only export with a .glb. The budget is what a new game gives characters/.",
            GameFormat::Glb,
            known("characters"),
        ),
        shape(
            GameTemplateId::LevelKit,
            "Level kit",
            "Modular pieces that repeat, as .glb",
            "A piece is drawn many times in a view, so it gets 5k triangles against the 12k of a prop.",
            GameFormat::Glb,
            kit(),
        ),
        shape(
            GameTemplateId::LevelKitObj,
            "Level kit, .obj",
            "The same pieces for an engine that reads .obj",
            "Static pieces need no rig or clips, so .obj with its .mtl carries all of them. Same budget as Level kit.",
            GameFormat::Obj,
            kit(),
        ),
        tuned(
            GameTemplateId::WebGame,
            Target,
            "Web game",
            "Small downloads for a browser game",
            format!(
                "Browsers download every model and texture, so everything is small: JPG textures (no transparency, use Keep for cutouts) capped at 1024 px. Game export writes plain files and no KTX2 or meshopt, so compress afterwards if you need to. No published limit applies here. {JUDGMENT}"
            ),
            jpg(1024, false),
            vec![
                row("props", 10_000, 1024),
                row("characters", 25_000, 1024),
                row("kit", 3_000, 512),
            ],
        ),
        tuned(
            GameTemplateId::Godot,
            Engine,
            "Godot",
            "A .glb project with room for PNG textures",
            format!(
                "Godot imports .glb files directly. PNG textures capped at 2048 px, with an environment/ folder for scenes. No published limit applies here. {JUDGMENT}"
            ),
            png(2048),
            vec![
                row("props", 20_000, 2048),
                row("characters", 50_000, 2048),
                row("environment", 30_000, 2048),
            ],
        ),
        tuned(
            GameTemplateId::Unity,
            Engine,
            "Unity",
            "Mid-range numbers for a desktop or console build",
            format!(
                "PNG textures capped at 2048 px, 1024 px on props and kit pieces. Check that your Unity project reads .glb before you ship. No published limit applies here. {JUDGMENT}"
            ),
            png(2048),
            vec![
                row("props", 15_000, 1024),
                row("characters", 40_000, 2048),
                row("kit", 5_000, 1024),
            ],
        ),
        tuned(
            GameTemplateId::Unreal,
            Engine,
            "Unreal",
            "High numbers for a desktop or console build",
            format!(
                "PNG textures capped at 4096 px, an environment/ folder for scenes. Check that your Unreal project imports .glb before you ship. No published limit applies here. {JUDGMENT}"
            ),
            png(4096),
            vec![
                row("props", 40_000, 2048),
                row("characters", 80_000, 4096),
                row("environment", 60_000, 2048),
            ],
        ),
        tuned(
            GameTemplateId::Mobile,
            Target,
            "Mobile",
            "Small models and 512 px textures for phones",
            format!(
                "Phones have little memory and bandwidth, so models and textures stay small: JPG textures capped at 1024 px, 512 px on props and kit pieces. No published limit applies here. {JUDGMENT}"
            ),
            jpg(1024, false),
            vec![
                row("props", 4_000, 512),
                row("characters", 15_000, 1024),
                row("kit", 2_000, 512),
            ],
        ),
        tuned(
            GameTemplateId::VrStandalone,
            Target,
            "VR standalone",
            "Headsets that render on their own chip",
            "A standalone headset draws the scene twice and has a small budget. Meta's device comparison page lists whole scene limits of about 1.5M triangles and 200 draw calls on Quest 3 and 750K and 100 on Quest 2. That is a limit for the whole view, not a model. The per model rows here are judgment, picked so a view of a few dozen models stays under it. JPG textures capped at 1024 px.".to_string(),
            jpg(1024, false),
            vec![
                row("props", 5_000, 1024),
                row("characters", 20_000, 1024),
                row("kit", 2_000, 512),
            ],
        ),
        tuned(
            GameTemplateId::GameJam,
            Target,
            "Game jam",
            "One folder, small files, no extra maps",
            format!(
                "For a few days of work: every model in the model folder checks against one budget of 20k triangles and 1024 px, JPG textures, and normal and metal-roughness maps left out so files stay small. A folder with its own budget keeps it. No published limit applies here. {JUDGMENT}"
            ),
            jpg(1024, true),
            vec![row("", 20_000, 1024)],
        ),
    ]
}

/// The template `id` names.
pub fn game_template(id: GameTemplateId) -> GameTemplate {
    game_templates()
        .into_iter()
        .find(|t| t.id == id)
        .expect("every id is in the list")
}

/// `current` with `rows` laid over it: a folder in both takes the template's
/// numbers, every other row stays, since a folder a template does not name
/// still has models that check against its budget.
pub fn merge_budgets(current: &[FolderBudget], rows: &[FolderBudget]) -> Vec<FolderBudget> {
    let mut out = current.to_vec();
    for row in rows {
        match out.iter_mut().find(|b| b.folder == row.folder) {
            Some(found) => *found = row.clone(),
            None => out.push(row.clone()),
        }
    }
    out
}
