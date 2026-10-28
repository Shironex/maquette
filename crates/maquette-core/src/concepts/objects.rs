//! Object refs: one isolated image per object of a concept, the kind of
//! picture a 3D service turns into a clean model.
//!
//! You list the objects in a concept ("desk lamp", "plaster plinth with
//! clay figure"), and each one becomes one Codex image job on the concept
//! lane, with the concept attached as the reference. The images land in a
//! new direction named after the source ("Workbench hero objects"), one per
//! object, each titled by its object. Another source with the same name
//! gets its own direction ("Workbench hero objects 2"), so neither loses
//! its source or kind. The prompt template lives here and nowhere else.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::codex::assemble;
use super::style::StyleBoard;
use super::{
    Aspect, Direction, MAX_IMAGES_PER_BATCH, NewConcepts, REFS_DIR, load_direction,
    modify_direction, resolve_ref,
};
use crate::assets::{self, Asset, AssetKind};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest, slugify};
use crate::workspace::{FolderLayout, is_single_segment};

/// The most objects one request makes, the same cap as a concept batch.
pub const MAX_OBJECTS: usize = MAX_IMAGES_PER_BATCH as usize;
const MAX_OBJECT_CHARS: usize = 60;
const MAX_NOTE_CHARS: usize = 400;
const MAX_DIRECTION_CHARS: usize = 60;
const SUFFIX: &str = " objects";

/// What "Make object refs" asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewObjectRefs {
    pub project: String,
    /// The concept to take the objects from, relative to the project folder
    /// (`concepts/...` or `refs/...`).
    pub source: String,
    /// The direction or asset the concept belongs to. The new direction is
    /// called "<source name> objects".
    pub source_name: String,
    /// One name per object, for example "desk lamp".
    pub objects: Vec<String>,
    /// Extra instructions for the Codex prompt, by object name (any case).
    /// They steer the image only: the asset is named after the object.
    #[serde(default)]
    pub notes: HashMap<String, String>,
    pub aspect: Aspect,
    /// What the objects become on the Board: props or characters.
    pub kind: AssetKind,
    /// Whether the jobs carry the project's style board, on by default.
    #[serde(default = "super::style_on")]
    pub use_style: bool,
}

/// One object asset that Send picks made or moved to Refs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ObjectAsset {
    pub asset: Asset,
    /// Its ref in `refs/`, relative to the project folder.
    pub refs: Vec<String>,
}

/// What Send picks did for a direction of object refs: one asset per pick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SentObjects {
    pub direction: Direction,
    pub assets: Vec<ObjectAsset>,
    /// The new files in `refs/`.
    pub copied: Vec<String>,
    pub project: ProjectManifest,
}

/// "Workbench hero objects", kept within the direction name limit.
pub fn direction_name(source_name: &str) -> String {
    numbered_name(source_name, 1)
}

/// "Workbench hero objects" for 1, "Workbench hero objects 2" after that,
/// kept within the direction name limit.
fn numbered_name(source_name: &str, n: u32) -> String {
    let base = source_name.trim();
    let base = if base.is_empty() { "Concept" } else { base };
    let suffix = if n == 1 {
        SUFFIX.to_string()
    } else {
        format!("{SUFFIX} {n}")
    };
    let room = MAX_DIRECTION_CHARS - suffix.chars().count();
    let base: String = base.chars().take(room).collect();
    format!("{}{suffix}", base.trim_end())
}

/// The name of the direction a request's images go to: the first of
/// "<source name> objects", "... objects 2", "... objects 3" that is free
/// or already holds object refs of this very source and kind. A direction
/// of another source (or a plain one that happens to share the name) is
/// left alone, so its source and kind stay as they are.
fn objects_direction_name(project_dir: &Path, new: &NewObjectRefs) -> CoreResult<String> {
    for n in 1..=99 {
        let name = numbered_name(&new.source_name, n);
        match load_direction(project_dir, &slugify(&name))? {
            None => return Ok(name),
            Some(d)
                if d.source.as_deref() == Some(new.source.as_str()) && d.kind == Some(new.kind) =>
            {
                return Ok(name);
            }
            Some(_) => {}
        }
    }
    Err(CoreError::Invalid(format!(
        "There are too many object directions named after {}. Give this one its own name.",
        new.source_name.trim()
    )))
}

/// One object per line or comma, trimmed, without repeats (any case).
pub fn split_objects(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in text.split(['\n', ',', ';']) {
        let name = part.split_whitespace().collect::<Vec<_>>().join(" ");
        if !name.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&name)) {
            out.push(name);
        }
    }
    out
}

pub(super) fn check_source(project_dir: &Path, source: &str) -> CoreResult<String> {
    let clean = source.trim().replace('\\', "/");
    let inside = !clean.is_empty()
        && !clean.starts_with('/')
        && !clean.contains(':')
        && clean
            .split('/')
            .all(|part| is_single_segment(part) && !part.is_empty());
    if !inside {
        return Err(CoreError::Invalid(format!(
            "{source} is not an image in this project."
        )));
    }
    if !resolve_ref(project_dir, &clean).is_file() {
        return Err(CoreError::NotFound(format!("the concept {clean}")));
    }
    Ok(clean)
}

/// Checks a request and returns it cleaned.
pub fn clean(project_dir: &Path, new: &NewObjectRefs) -> CoreResult<NewObjectRefs> {
    let source = check_source(project_dir, &new.source)?;
    let objects = split_objects(&new.objects.join("\n"));
    if objects.is_empty() {
        return Err(CoreError::Invalid(
            "List at least one object, for example \"desk lamp\".".to_string(),
        ));
    }
    if objects.len() > MAX_OBJECTS {
        return Err(CoreError::Invalid(format!(
            "Make {MAX_OBJECTS} object refs at most in one go."
        )));
    }
    if let Some(long) = objects
        .iter()
        .find(|o| o.chars().count() > MAX_OBJECT_CHARS)
    {
        return Err(CoreError::Invalid(format!(
            "Keep each object to {MAX_OBJECT_CHARS} characters or fewer: \"{long}\" is longer."
        )));
    }
    if new.kind == AssetKind::Scene {
        return Err(CoreError::Invalid(
            "Object refs are props or characters, not scenes.".to_string(),
        ));
    }
    let notes = clean_notes(&objects, &new.notes)?;
    Ok(NewObjectRefs {
        project: new.project.clone(),
        source,
        source_name: new.source_name.trim().to_string(),
        objects,
        notes,
        aspect: new.aspect,
        kind: new.kind,
        use_style: new.use_style,
    })
}

/// The notes that belong to a listed object, trimmed to one line and keyed
/// by the object's own spelling. Notes for other names and empty ones drop.
fn clean_notes(
    objects: &[String],
    notes: &HashMap<String, String>,
) -> CoreResult<HashMap<String, String>> {
    let mut out = HashMap::new();
    for (key, text) in notes {
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let key = key.trim();
        let Some(object) = objects.iter().find(|o| o.eq_ignore_ascii_case(key)) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        if text.chars().count() > MAX_NOTE_CHARS {
            return Err(CoreError::Invalid(format!(
                "Keep each note to {MAX_NOTE_CHARS} characters or fewer: the note for \"{object}\" is longer."
            )));
        }
        out.insert(object.clone(), text);
    }
    Ok(out)
}

/// The prompt for one object. Codex gets the source concept as the first
/// `-i`, so "the attached concept" is that image; a style board's images
/// come after it.
pub fn object_prompt(
    object: &str,
    notes: &str,
    kind: AssetKind,
    source_name: &str,
    aspect: Aspect,
    out: &Path,
    style: Option<&StyleBoard>,
) -> String {
    let object = object.trim();
    let notes = notes.trim();
    let (what, pose) = match kind {
        AssetKind::Character => (
            "character",
            " Show the whole figure from head to feet, standing, arms a little away from the body.",
        ),
        AssetKind::Prop | AssetKind::Scene => ("object", ""),
    };
    let image = if style.is_some_and(|s| !s.images.is_empty()) {
        "the first attached image, the concept"
    } else {
        "the attached concept image"
    };
    let from = if source_name.trim().is_empty() {
        image.to_string()
    } else {
        format!("{image} (the direction \"{}\")", source_name.trim())
    };
    let extra = if notes.is_empty() {
        String::new()
    } else {
        format!("Extra instructions for this {what}: {notes}\n\n")
    };
    let lead = format!(
        "Make a clean reference image of one {what} for a 3D model: the \"{object}\" from {from}.\n\n\
         Show only the {object}, isolated: centered, the whole {what} in frame with a little space around it, \
         seen from a front three-quarter view.{pose} Use a plain light grey background, even soft light \
         and no cast shadow on the background. No other objects, no text, no labels, no watermark. \
         Match the source's materials, colours, proportions and style, so it reads as the same {what} \
         as in the concept.\n\n\
         {extra}Use a {ratio} {shape} composition.",
        ratio = aspect.ratio(),
        shape = aspect.shape(),
    );
    assemble(&lead, style, out)
}

/// One image an objects request will make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedObject {
    pub index: u32,
    /// Relative to the project folder.
    pub file: String,
    pub object: String,
    /// Extra instructions for the prompt, empty for none.
    pub notes: String,
}

/// Creates the objects direction (or adds to it) and returns it with the
/// planned images, in the order given.
pub fn prepare(
    project_dir: &Path,
    new: &NewObjectRefs,
) -> CoreResult<(Direction, Vec<PlannedObject>)> {
    prepare_with(project_dir, new, None)
}

/// [`prepare`] for a request that carries the style board at
/// `style_revision`, which the direction records.
pub(crate) fn prepare_with(
    project_dir: &Path,
    new: &NewObjectRefs,
    style_revision: Option<u32>,
) -> CoreResult<(Direction, Vec<PlannedObject>)> {
    let name = objects_direction_name(project_dir, new)?;
    #[expect(clippy::cast_possible_truncation, reason = "at most 12 objects")]
    let count = new.objects.len() as u32;
    let brief = NewConcepts {
        project: new.project.clone(),
        direction: name,
        prompt: format!(
            "Object refs from {}: {}.",
            new.source.rsplit('/').next().unwrap_or(&new.source),
            new.objects.join(", ")
        ),
        refs: vec![new.source.clone()],
        aspect: new.aspect,
        count,
        use_style: new.use_style,
    };
    let (direction, files) = super::prepare_with(project_dir, &brief, style_revision)?;
    let (direction, ()) = modify_direction(project_dir, &direction.id, |d| {
        d.source = Some(new.source.clone());
        d.kind = Some(new.kind);
        Ok(())
    })?;
    let planned = files
        .into_iter()
        .zip(&new.objects)
        .map(|((index, file), object)| PlannedObject {
            index,
            file,
            object: object.clone(),
            notes: new.notes.get(object).cloned().unwrap_or_default(),
        })
        .collect();
    Ok((direction, planned))
}

/// `refs/desk_lamp.png`, or `refs/desk_lamp_2.png` when that name holds
/// another file.
pub(super) fn ref_name(project_dir: &Path, title: &str, ext: &str) -> String {
    free_name(project_dir, REFS_DIR, title, ext)
}

/// `<dir>/desk_lamp.png`, or `<dir>/desk_lamp_2.png` when that name holds
/// another file. The name is slugged, so it stays one segment in `dir`.
pub(crate) fn free_name(project_dir: &Path, dir: &str, title: &str, ext: &str) -> String {
    let stem = slugify(title).replace('-', "_");
    let stem = if stem.is_empty() { "object" } else { &stem };
    (1u32..)
        .map(|n| {
            if n == 1 {
                format!("{dir}/{stem}.{ext}")
            } else {
                format!("{dir}/{stem}_{n}.{ext}")
            }
        })
        .find(|rel| !project_dir.join(rel).exists())
        .unwrap_or_else(|| format!("{dir}/{stem}.{ext}"))
}

/// The object an image shows: its title, else the direction and number.
pub(super) fn image_title(direction: &Direction, image: &super::ConceptImage) -> String {
    image
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| format!("{} {}", direction.name, image.index))
}

/// Send picks for a direction of object refs: each picked image goes into
/// `refs/` named after its object and becomes its own asset at Refs (a new
/// one, or an existing one moved up from Concept). `kind` overrides the
/// direction's kind.
pub fn send_object_picks(
    workspace_root: &Path,
    layout: &FolderLayout,
    project_id: &str,
    direction_id: &str,
    kind: Option<AssetKind>,
) -> CoreResult<SentObjects> {
    let project_dir = project::dir(workspace_root, layout, project_id)?;
    let current = load_direction(&project_dir, direction_id)?
        .ok_or_else(|| CoreError::NotFound(format!("the direction {direction_id}")))?;
    if !current.images.iter().any(|i| i.picked) {
        return Err(CoreError::Invalid(
            "Pick at least one object to send.".to_string(),
        ));
    }
    let kind = kind.or(current.kind).unwrap_or(AssetKind::Prop);
    let refs_dir = project_dir.join(REFS_DIR);
    fs::create_dir_all(&refs_dir).map_err(|e| CoreError::io(&refs_dir, e))?;
    let (direction, (sent, copied)) = modify_direction(&project_dir, direction_id, |d| {
        let name = d.name.clone();
        let mut sent: Vec<(String, String)> = Vec::new();
        let mut copied = Vec::new();
        let titles: Vec<String> = d.images.iter().map(|i| image_title(d, i)).collect();
        for (image, title) in d.images.iter_mut().zip(titles) {
            if !image.picked {
                continue;
            }
            let kept = image
                .ref_file
                .clone()
                .filter(|r| project_dir.join(r).is_file());
            let rel = match kept {
                Some(rel) => rel,
                None => {
                    let from = project_dir.join(&image.file);
                    let ext = from
                        .extension()
                        .map_or_else(|| "png".to_string(), |e| e.to_string_lossy().to_lowercase());
                    let rel = ref_name(&project_dir, &title, &ext);
                    fs::copy(&from, project_dir.join(&rel)).map_err(|e| CoreError::io(&from, e))?;
                    image.ref_file = Some(rel.clone());
                    copied.push(rel.clone());
                    rel
                }
            };
            if sent.iter().any(|(t, _)| t.eq_ignore_ascii_case(&title)) {
                return Err(CoreError::Invalid(format!(
                    "Two picks in {name} are both called {title}. Unpick one of them."
                )));
            }
            sent.push((title, rel));
        }
        Ok((sent, copied))
    })?;
    let mut placed: Vec<ObjectAsset> = Vec::new();
    let manifest = project::modify(workspace_root, layout, project_id, |manifest| {
        placed.clear();
        for (title, rel) in &sent {
            let asset = super::picks::place_at_refs(manifest, title, kind)?;
            assets::set_ref_thumbnail(manifest, &asset.id, rel)?;
            placed.push(ObjectAsset {
                asset: assets::find(manifest, &asset.id)?.clone(),
                refs: vec![rel.clone()],
            });
        }
        Ok(())
    })?;
    Ok(SentObjects {
        direction,
        assets: placed,
        copied,
        project: manifest,
    })
}

#[cfg(test)]
mod tests;
