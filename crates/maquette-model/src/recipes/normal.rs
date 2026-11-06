//! Reading a recipe file and the shape it is written in: the file version
//! it needs, trimmed names, and each change and time edit holding only the
//! fields its kind reads.

use std::io;
use std::path::Path;

use super::{
    BlankKind, LayerKind, LayerSettings, MAX_FILE_BYTES, RECIPE_VERSION, RECIPE_VERSION_1,
    REST_POSE_NAME, RecipeBase, RecipeBlank, RecipeFile, edits, keys, rest_length, uses_v2,
    validate, version_of,
};
use crate::error::{CoreError, CoreResult};
use crate::plans::trimmed;

/// [`load_file`], with the file's [`version_of`].
///
/// # Errors
///
/// As [`load_file`].
pub fn load_versioned(file: &Path) -> CoreResult<(RecipeFile, String)> {
    let text = match crate::plans::read_capped(file, MAX_FILE_BYTES) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound(format!(
                "the recipe file {}",
                file.display()
            )));
        }
        Err(err) if err.kind() == io::ErrorKind::FileTooLarge => {
            return Err(CoreError::Invalid(format!(
                "The recipe file is over {} KB, too big to be a recipe.",
                MAX_FILE_BYTES / 1024
            )));
        }
        Err(err) => return Err(CoreError::io(file, err)),
    };
    let version = version_of(&text);
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| CoreError::json(file, e))?;
    // The version first, so a newer file whose shape changed says so.
    let found = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if found > u64::from(RECIPE_VERSION) {
        return Err(CoreError::UnsupportedVersion {
            found: u32::try_from(found).unwrap_or(u32::MAX),
            supported: RECIPE_VERSION,
        });
    }
    let recipe: RecipeFile = serde_json::from_value(value).map_err(|e| CoreError::json(file, e))?;
    let recipe = normalized(&recipe);
    validate(&recipe)?;
    Ok((recipe, version))
}

/// The recipe as it is written: version 1 while it uses nothing of version
/// 2 ([`uses_v2`]) and 2 once it does, a trimmed name, clip names and
/// labels, each change holding only the settings and fields its kind reads,
/// pose keys in clip and frame order, and each time edit only the fields
/// its kind reads.
pub fn normalized(recipe: &RecipeFile) -> RecipeFile {
    let blanks = recipe
        .blanks
        .iter()
        .map(|b| RecipeBlank {
            id: trimmed(&b.id).to_string(),
            label: trimmed(&b.label).to_string(),
            prop: if b.kind == BlankKind::Prop {
                b.prop.as_deref().map(|p| trimmed(p).to_string())
            } else {
                None
            },
            point: if b.kind == BlankKind::Point {
                b.point
            } else {
                None
            },
            kind: b.kind,
        })
        .collect();
    let layers = recipe.layers.iter().map(keys::layer_for).collect();
    let mut out = RecipeFile {
        version: RECIPE_VERSION_1,
        name: trimmed(&recipe.name).to_string(),
        base: RecipeBase {
            blend: None,
            ..clip_for(&recipe.base)
        },
        joined: recipe
            .joined
            .iter()
            .map(|c| RecipeBase {
                blend: Some(c.blend.clone().unwrap_or_default()),
                ..clip_for(c)
            })
            .collect(),
        time: recipe.time.iter().map(edits::time_for).collect(),
        blanks,
        layers,
        ..recipe.clone()
    };
    if uses_v2(&out) {
        out.version = RECIPE_VERSION;
    }
    out
}

/// A clip as it is written: the rest pose's fixed name, length and hash, or
/// a trimmed name and the by name mark only where it holds.
fn clip_for(clip: &RecipeBase) -> RecipeBase {
    if clip.rest {
        RecipeBase {
            name: REST_POSE_NAME.to_string(),
            frames: clip.frames,
            length: rest_length(clip.frames),
            hash: String::new(),
            mixamo: false,
            rest: true,
            by_name: false,
            trim: clip.trim,
            blend: clip.blend.clone(),
        }
    } else {
        RecipeBase {
            name: trimmed(&clip.name).to_string(),
            by_name: clip.name_only(),
            ..clip.clone()
        }
    }
}

/// `settings` with only the fields `kind` reads.
pub(super) fn settings_for(kind: LayerKind, settings: &LayerSettings) -> LayerSettings {
    let targeted = matches!(
        kind,
        LayerKind::Reach | LayerKind::LookAt | LayerKind::Pin | LayerKind::Grip
    );
    let nudge = kind == LayerKind::Nudge;
    let wave = kind == LayerKind::Wave;
    let eased = matches!(kind, LayerKind::Pin | LayerKind::Grip);
    let whole = kind == LayerKind::Pose;
    LayerSettings {
        weight: settings.weight,
        start: if whole { 0.0 } else { settings.start },
        end: if whole { 1.0 } else { settings.end },
        target: settings
            .target
            .as_deref()
            .map(|t| trimmed(t).to_string())
            .filter(|_| targeted),
        rotation: settings.rotation.filter(|_| nudge),
        offset: settings.offset.filter(|_| nudge),
        cycles: settings.cycles.filter(|_| wave),
        amount: settings.amount.filter(|_| wave),
        ease_in: settings.ease_in.filter(|_| eased),
        ease_out: settings.ease_out.filter(|_| eased),
        flat: settings.flat.filter(|_| kind == LayerKind::Pin),
    }
}
