//! Object refs: one isolated image per object of a concept, the kind of
//! picture a 3D service turns into a clean model.
//!
//! You list the objects in a concept ("desk lamp", "plaster plinth with
//! clay figure"), and each one becomes one Codex image job on the concept
//! lane, with the concept attached as the reference. The images land in a
//! new direction named after the source ("Workbench hero objects"), one per
//! object, each titled by its object. The prompt template lives here and
//! nowhere else.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    Aspect, Direction, MAX_IMAGES_PER_BATCH, NewConcepts, REFS_DIR, load_direction,
    modify_direction, resolve_ref, save_instructions,
};
use crate::assets::{self, Asset, AssetKind, NewAsset, Stage};
use crate::error::{CoreError, CoreResult};
use crate::project::{self, ProjectManifest, slugify};
use crate::workspace::{FolderLayout, is_single_segment};

/// The most objects one request makes, the same cap as a concept batch.
pub const MAX_OBJECTS: usize = MAX_IMAGES_PER_BATCH as usize;
const MAX_OBJECT_CHARS: usize = 60;
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
    pub aspect: Aspect,
    /// What the objects become on the Board: props or characters.
    pub kind: AssetKind,
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
    let base = source_name.trim();
    let base = if base.is_empty() { "Concept" } else { base };
    let room = MAX_DIRECTION_CHARS - SUFFIX.chars().count();
    let base: String = base.chars().take(room).collect();
    format!("{}{SUFFIX}", base.trim_end())
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

fn check_source(project_dir: &Path, source: &str) -> CoreResult<String> {
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
    Ok(NewObjectRefs {
        project: new.project.clone(),
        source,
        source_name: new.source_name.trim().to_string(),
        objects,
        aspect: new.aspect,
        kind: new.kind,
    })
}

/// The prompt for one object. Codex gets the source concept as `-i`, so
/// "the attached concept" is that image.
pub fn object_prompt(
    object: &str,
    kind: AssetKind,
    source_name: &str,
    aspect: Aspect,
    out: &Path,
) -> String {
    let object = object.trim();
    let (what, pose) = match kind {
        AssetKind::Character => (
            "character",
            " Show the whole figure from head to feet, standing, arms a little away from the body.",
        ),
        AssetKind::Prop | AssetKind::Scene => ("object", ""),
    };
    let from = if source_name.trim().is_empty() {
        "the attached concept image".to_string()
    } else {
        format!(
            "the attached concept image (the direction \"{}\")",
            source_name.trim()
        )
    };
    format!(
        "Generate an image.\n\n\
         Make a clean reference image of one {what} for a 3D model: the \"{object}\" from {from}.\n\n\
         Show only the {object}, isolated: centered, the whole {what} in frame with a little space around it, \
         seen from a front three-quarter view.{pose} Use a plain light grey background, even soft light \
         and no cast shadow on the background. No other objects, no text, no labels, no watermark. \
         Match the source's materials, colours, proportions and style, so it reads as the same {what} \
         as in the concept.\n\n\
         Use a {ratio} {shape} composition.\n\n{save}",
        ratio = aspect.ratio(),
        shape = aspect.shape(),
        save = save_instructions(out),
    )
}

/// One image an objects request will make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedObject {
    pub index: u32,
    /// Relative to the project folder.
    pub file: String,
    pub object: String,
}

/// Creates the objects direction (or adds to it) and returns it with the
/// planned images, in the order given.
pub fn prepare(
    project_dir: &Path,
    new: &NewObjectRefs,
) -> CoreResult<(Direction, Vec<PlannedObject>)> {
    let name = direction_name(&new.source_name);
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
    };
    let (direction, files) = super::prepare(project_dir, &brief)?;
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
        })
        .collect();
    Ok((direction, planned))
}

/// `refs/desk_lamp.png`, or `refs/desk_lamp_2.png` when that name holds
/// another file.
fn ref_name(project_dir: &Path, title: &str, ext: &str) -> String {
    let stem = slugify(title).replace('-', "_");
    let stem = if stem.is_empty() { "object" } else { &stem };
    (1u32..)
        .map(|n| {
            if n == 1 {
                format!("{REFS_DIR}/{stem}.{ext}")
            } else {
                format!("{REFS_DIR}/{stem}_{n}.{ext}")
            }
        })
        .find(|rel| !project_dir.join(rel).exists())
        .unwrap_or_else(|| format!("{REFS_DIR}/{stem}.{ext}"))
}

/// The object an image shows: its title, else the direction and number.
fn image_title(direction: &Direction, image: &super::ConceptImage) -> String {
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
            let found = assets::find_by_name(manifest, title).cloned();
            let asset = match found {
                Some(existing) if existing.stage < Stage::Refs => {
                    assets::move_to(manifest, &existing.id, Stage::Refs)?
                }
                Some(existing) => existing,
                None => assets::add(
                    manifest,
                    NewAsset {
                        name: title.clone(),
                        kind,
                        stage: Stage::Refs,
                    },
                )?,
            };
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
mod tests {
    use super::*;
    use crate::concepts::{add_image, mark};

    fn request(objects: &[&str]) -> NewObjectRefs {
        NewObjectRefs {
            project: "landing".into(),
            source: "concepts/workbench-hero/workbench-hero_01.png".into(),
            source_name: "Workbench hero".into(),
            objects: objects.iter().map(|o| (*o).to_string()).collect(),
            aspect: Aspect::Square,
            kind: AssetKind::Prop,
        }
    }

    #[test]
    fn the_object_prompt_asks_for_one_isolated_object() {
        let out =
            Path::new("/w/landing/concepts/workbench-hero-objects/workbench-hero-objects_01.png");
        let text = object_prompt(
            "desk lamp",
            AssetKind::Prop,
            "Workbench hero",
            Aspect::Square,
            out,
        );
        assert!(text.starts_with("Generate an image.\n\n"));
        assert!(text.contains(
            "Make a clean reference image of one object for a 3D model: the \"desk lamp\" from the attached concept image (the direction \"Workbench hero\")."
        ));
        for wanted in [
            "Show only the desk lamp, isolated: centered,",
            "the whole object in frame",
            "front three-quarter view",
            "plain light grey background",
            "even soft light",
            "no cast shadow on the background",
            "No other objects, no text, no labels, no watermark.",
            "Match the source's materials, colours, proportions and style",
            "Use a 1:1 square composition.",
            "~/.codex/generated_images/",
        ] {
            assert!(text.contains(wanted), "missing {wanted:?} in\n{text}");
        }
        assert!(!text.contains("head to feet"));
        assert!(text.trim_end().ends_with(&out.display().to_string()));
        assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'));

        let figure = object_prompt(
            "clay figure",
            AssetKind::Character,
            "",
            Aspect::Portrait,
            out,
        );
        assert!(figure.contains("one character for a 3D model"));
        assert!(figure.contains("from the attached concept image."));
        assert!(figure.contains("the whole character in frame"));
        assert!(figure.contains("from head to feet"));
        assert!(figure.contains("3:4 portrait"));
    }

    #[test]
    fn object_lists_are_split_and_cleaned() {
        assert_eq!(
            split_objects("desk lamp\n  plaster   plinth with clay figure ,laptop;\nDesk lamp\n\n"),
            vec!["desk lamp", "plaster plinth with clay figure", "laptop"]
        );
        assert_eq!(
            direction_name("  Workbench hero "),
            "Workbench hero objects"
        );
        assert_eq!(direction_name(""), "Concept objects");
        let long = "x".repeat(80);
        assert_eq!(direction_name(&long).chars().count(), 60);
    }

    #[test]
    fn requests_are_checked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("concepts/workbench-hero");
        fs::create_dir_all(&src).expect("mkdir");
        fs::write(src.join("workbench-hero_01.png"), b"png").expect("png");
        let ok = clean(dir.path(), &request(&["desk lamp", " laptop ", "Laptop"])).expect("ok");
        assert_eq!(ok.objects, vec!["desk lamp", "laptop"]);
        assert!(clean(dir.path(), &request(&[])).is_err());
        assert!(clean(dir.path(), &request(&[&"y".repeat(61)])).is_err());
        let many: Vec<String> = (0..13).map(|n| format!("thing {n}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        assert!(clean(dir.path(), &request(&many)).is_err());
        let mut outside = request(&["lamp"]);
        outside.source = "../elsewhere.png".into();
        assert!(clean(dir.path(), &outside).is_err());
        outside.source = "/abs/elsewhere.png".into();
        assert!(clean(dir.path(), &outside).is_err());
        let mut missing = request(&["lamp"]);
        missing.source = "refs/missing.png".into();
        assert!(clean(dir.path(), &missing).is_err());
        let mut scene = request(&["lamp"]);
        scene.kind = AssetKind::Scene;
        assert!(clean(dir.path(), &scene).is_err());
    }

    #[test]
    fn prepare_names_the_direction_after_the_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (direction, planned) =
            prepare(dir.path(), &request(&["desk lamp", "laptop"])).expect("prep");
        assert_eq!(direction.name, "Workbench hero objects");
        assert_eq!(direction.id, "workbench-hero-objects");
        assert_eq!(direction.aspect, Aspect::Square);
        assert_eq!(direction.kind, Some(AssetKind::Prop));
        assert!(direction.is_objects());
        assert_eq!(
            direction.refs,
            vec!["concepts/workbench-hero/workbench-hero_01.png".to_string()]
        );
        assert_eq!(
            planned,
            vec![
                PlannedObject {
                    index: 1,
                    file: "concepts/workbench-hero-objects/workbench-hero-objects_01.png".into(),
                    object: "desk lamp".into(),
                },
                PlannedObject {
                    index: 2,
                    file: "concepts/workbench-hero-objects/workbench-hero-objects_02.png".into(),
                    object: "laptop".into(),
                },
            ]
        );
    }

    #[test]
    fn send_object_picks_makes_one_asset_per_object() {
        let root = tempfile::tempdir().expect("tempdir");
        let layout = FolderLayout::default();
        let manifest = project::create(
            root.path(),
            &layout,
            crate::project::NewProject {
                name: "Landing".into(),
                template: crate::project::ProjectTemplate::Room,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        let project_dir = project::dir(root.path(), &layout, &manifest.id).expect("dir");
        let (direction, planned) = prepare(
            &project_dir,
            &request(&["desk lamp", "laptop", "brush cup"]),
        )
        .expect("prep");
        let png = crate::inbox::images::write_png(2, 2, &[90; 12]);
        for p in &planned {
            fs::write(project_dir.join(&p.file), &png).expect("png");
            add_image(
                &project_dir,
                &direction.id,
                p.index,
                &p.file,
                Some(&p.object),
            )
            .expect("add");
        }
        assert!(
            send_object_picks(root.path(), &layout, &manifest.id, &direction.id, None).is_err(),
            "nothing picked"
        );
        // Someone already keeps a laptop.png in refs/.
        fs::write(project_dir.join("refs/laptop.png"), b"mine").expect("theirs");
        for p in &planned[..2] {
            mark(&project_dir, &direction.id, &p.file, None, Some(true)).expect("pick");
        }
        let sent = send_object_picks(root.path(), &layout, &manifest.id, &direction.id, None)
            .expect("send");
        let names: Vec<&str> = sent.assets.iter().map(|a| a.asset.name.as_str()).collect();
        assert_eq!(names, vec!["desk lamp", "laptop"]);
        assert_eq!(sent.assets[0].refs, vec!["refs/desk_lamp.png".to_string()]);
        assert_eq!(sent.assets[1].refs, vec!["refs/laptop_2.png".to_string()]);
        assert_eq!(
            fs::read(project_dir.join("refs/laptop.png")).expect("kept"),
            b"mine"
        );
        assert!(sent.assets.iter().all(|a| a.asset.stage == Stage::Refs
            && a.asset.kind == AssetKind::Prop
            && a.asset.thumbnail.as_deref() == a.refs.first().map(String::as_str)));
        assert_eq!(sent.copied.len(), 2);
        assert_eq!(
            sent.direction.images[0].ref_file.as_deref(),
            Some("refs/desk_lamp.png")
        );

        // Again, as characters: no new copies, the same assets.
        let again = send_object_picks(
            root.path(),
            &layout,
            &manifest.id,
            &direction.id,
            Some(AssetKind::Character),
        )
        .expect("again");
        assert!(again.copied.is_empty());
        assert_eq!(again.assets[0].asset.id, sent.assets[0].asset.id);
        let saved = project::load(root.path(), &layout, &manifest.id).expect("load");
        assert_eq!(saved.assets.len(), 2);
    }
}
