//! Game templates: a short built-in list of export settings for the usual
//! kinds of game asset. A template only fills settings a person can edit by
//! hand, the model format and budget rows per folder; it brings no assets
//! and no game source, and the project does not remember it.

use serde::{Deserialize, Serialize};

use super::{FolderBudget, GameFormat, default_budgets};

/// One of the built-in templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum GameTemplateId {
    PropSet,
    Character,
    LevelKit,
    LevelKitObj,
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
    pub format: GameFormat,
    /// The folders it lays out, with the budget of each.
    pub budgets: Vec<FolderBudget>,
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

/// The templates, in the order the app lists them.
pub fn game_templates() -> Vec<GameTemplate> {
    let make = |id, name: &str, detail: &str, why: &str, format, budgets| GameTemplate {
        id,
        name: name.to_string(),
        detail: detail.to_string(),
        why: why.to_string(),
        format,
        budgets,
    };
    vec![
        make(
            GameTemplateId::PropSet,
            "Prop set",
            "One file per prop, no scene",
            "A .glb holds a prop and its textures in one file. The budget is what a new game gives props/.",
            GameFormat::Glb,
            known("props"),
        ),
        make(
            GameTemplateId::Character,
            "Character",
            "One rigged model with its clips",
            "Rigs and clips only export with a .glb. The budget is what a new game gives characters/.",
            GameFormat::Glb,
            known("characters"),
        ),
        make(
            GameTemplateId::LevelKit,
            "Level kit",
            "Modular pieces that repeat, as .glb",
            "A piece is drawn many times in a view, so it gets 5k triangles against the 12k of a prop.",
            GameFormat::Glb,
            kit(),
        ),
        make(
            GameTemplateId::LevelKitObj,
            "Level kit, .obj",
            "The same pieces for an engine that reads .obj",
            "Static pieces need no rig or clips, so .obj with its .mtl carries all of them. Same budget as Level kit.",
            GameFormat::Obj,
            kit(),
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
