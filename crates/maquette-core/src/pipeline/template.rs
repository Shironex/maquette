//! Starting pipelines: one per project template, and a proposal built from
//! the versioned scripts of an imported folder.

mod proposal;

use std::path::Path;

use super::{CompressPreset, CompressSpec, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::project::{ProjectManifest, ProjectTemplate, scan_import};
use crate::workspace::{MeshoptLevel, TextureCodec};

pub use proposal::{ImportProposal, propose};

pub const CHARACTER_LOCK: &str =
    "Character stays on medium. High breaks her clips: the upper body spins further every frame.";

fn s(list: &[&str]) -> Vec<String> {
    list.iter().map(|v| v.to_string()).collect()
}

/// A headless Blender step. `blend` is the file Blender opens before the
/// script; `None` starts from the factory scene and leaves loading to the
/// script, which is how a step runs before its blend exists.
fn blender(
    id: &str,
    name: &str,
    script: &str,
    blend: Option<&str>,
    extra: &[&str],
    inputs: &[&str],
    outputs: &[&str],
    deps: &[&str],
) -> StepDef {
    let mut args = s(&["-b", "--factory-startup", "--python-exit-code", "1"]);
    args.extend(blend.map(str::to_string));
    args.extend(s(&["--python", "{script}", "--"]));
    args.extend(s(extra));
    StepDef {
        id: id.into(),
        name: name.into(),
        kind: StepKind::Blender,
        command: None,
        script: Some(script.into()),
        args,
        inputs: s(inputs),
        outputs: s(outputs),
        depends_on: s(deps),
        compress: None,
    }
}

fn compress(from: &str, to: &str, deps: &[&str]) -> StepDef {
    StepDef {
        id: "compress".into(),
        name: "Compress".into(),
        kind: StepKind::GltfTransform,
        command: None,
        script: None,
        args: Vec::new(),
        inputs: vec![format!("{from}/*.glb")],
        outputs: vec![format!("{to}/*.glb")],
        depends_on: s(deps),
        compress: Some(CompressSpec {
            from: from.into(),
            to: to.into(),
        }),
    }
}

/// Copies the shipped models and posters into the site repo's public
/// folder. Without a site repo in the project settings the script passes
/// and says so. It writes outside the pipeline root, so it has no outputs
/// and runs every time.
fn ship(deps: &[&str]) -> StepDef {
    StepDef {
        id: "ship".into(),
        name: "Ship".into(),
        kind: StepKind::Shell,
        command: Some("node".into()),
        script: Some("tools/ship.mjs".into()),
        args: s(&[
            "{script}",
            "--models",
            "web/dist",
            "--posters",
            "web/posters",
            "--site",
            "{siteRepo}",
            "--models-to",
            "public/models",
            "--posters-to",
            "public/posters",
        ]),
        inputs: s(&[
            "web/dist/*.glb",
            "web/posters/*.png",
            "web/posters/stops.json",
        ]),
        outputs: Vec::new(),
        depends_on: s(deps),
        compress: None,
    }
}

pub fn character_preset() -> CompressPreset {
    CompressPreset {
        model: "character".into(),
        label: "Character".into(),
        texture_size: 2048,
        data_texture_size: None,
        codec: TextureCodec::Etc1s,
        map_codecs: None,
        meshopt: MeshoptLevel::Medium,
        lock: Some(CHARACTER_LOCK.into()),
    }
}

/// The "Floating diorama" flow: build the scene from the models folder,
/// bake its light, export one GLB per model, compress them, render the
/// poster stops, ship to the site. The scripts in [`super::starter`] make
/// it run on a new project; the build step opens no blend, because
/// `blender/scene.blend` is optional.
fn diorama() -> Vec<StepDef> {
    vec![
        blender(
            "build",
            "Build scene",
            "tools/build_scene.py",
            None,
            &[
                "--base",
                "blender/scene.blend",
                "--models",
                "models",
                "--out",
                "blender/scene_built.blend",
            ],
            &[
                "blender/scene.blend",
                "models/*.glb",
                "models/*.gltf",
                LAYOUT_INPUT,
            ],
            &["blender/scene_built.blend"],
            &[],
        ),
        blender(
            "bake",
            "Bake light",
            "tools/bake_light.py",
            Some("blender/scene_built.blend"),
            &[
                "--out-dir",
                "blender/bake",
                "--out-blend",
                "blender/scene_baked.blend",
                "--size",
                "512",
                "--samples",
                "16",
            ],
            &["blender/scene_built.blend"],
            &["blender/bake/bake.json", "blender/scene_baked.blend"],
            &["build"],
        ),
        blender(
            "export",
            "Export GLB",
            "tools/export_web.py",
            Some("blender/scene_baked.blend"),
            &["--bake", "blender/bake", "--out", "web/raw"],
            &["blender/scene_baked.blend", "blender/bake/*"],
            &["web/raw/*.glb"],
            &["bake"],
        ),
        compress("web/raw", "web/dist", &["export"]),
        posters("blender/scene_baked.blend"),
        ship(&["compress", "posters"]),
    ]
}

/// Renders the tagged camera stops of `blend`, or one framed overview poster
/// when it has none. `stops.json` is always written, so the step passes.
fn posters(blend: &str) -> StepDef {
    blender(
        "posters",
        "Capture posters",
        "tools/render_stops.py",
        Some(blend),
        &["--out", "web/posters"],
        &[blend],
        &["web/posters/stops.json"],
        &["compress"],
    )
}

/// The "Character showcase" flow: build a lit, framed scene around the
/// newest character model, export it with its rig and clips, compress it,
/// render the poster stops, ship. Like the diorama, the build opens no
/// blend, because `blender/character.blend` is optional.
fn character_showcase() -> Vec<StepDef> {
    vec![
        blender(
            "build",
            "Build character",
            "tools/build_character.py",
            None,
            &[
                "--base",
                "blender/character.blend",
                "--models",
                "models",
                "--out",
                "blender/character_built.blend",
            ],
            &["blender/character.blend", "models/*.glb"],
            &["blender/character_built.blend"],
            &[],
        ),
        blender(
            "export",
            "Export GLB",
            "tools/export_character.py",
            Some("blender/character_built.blend"),
            &["--out", "web/raw"],
            &["blender/character_built.blend"],
            &["web/raw/*.glb"],
            &["build"],
        ),
        compress("web/raw", "web/dist", &["export"]),
        posters("blender/character_built.blend"),
        ship(&["compress", "posters"]),
    ]
}

/// The pipeline a new project of `template` starts with. Imported projects
/// start empty here; see [`propose`].
pub fn for_template(template: ProjectTemplate) -> PipelineDef {
    let steps = match template {
        ProjectTemplate::FloatingDiorama | ProjectTemplate::Room => diorama(),
        ProjectTemplate::CharacterShowcase => character_showcase(),
        ProjectTemplate::Imported => Vec::new(),
    };
    PipelineDef {
        version: PIPELINE_VERSION,
        root: None,
        steps,
        // Only a character showcase starts with the character row; a scene
        // gets one once it has a character (see [`fit_presets`]).
        presets: if template == ProjectTemplate::CharacterShowcase {
            vec![character_preset()]
        } else {
            Vec::new()
        },
    }
}

/// The scene layout the Build scene starter reads, so moving a model or a
/// camera in the app rebuilds the scene.
pub const LAYOUT_INPUT: &str = crate::layout::LAYOUT_FILE;
const BUILD_SCRIPT: &str = "tools/build_scene.py";

/// Adds the scene layout to the inputs of a step that runs the Build scene
/// starter once that script reads it, so a project made before layouts
/// rebuilds when its layout changes after its starters were updated.
/// Returns true when `def` changed.
pub fn fit_layout_input(def: &mut PipelineDef, project_dir: &Path) -> bool {
    let root = def.root_dir(project_dir);
    let mut changed = false;
    for step in &mut def.steps {
        if step.script.as_deref() != Some(BUILD_SCRIPT)
            || step.inputs.iter().any(|i| i == LAYOUT_INPUT)
        {
            continue;
        }
        let reads = std::fs::read_to_string(root.join(BUILD_SCRIPT))
            .is_ok_and(|text| text.contains(LAYOUT_INPUT));
        if reads {
            step.inputs.push(LAYOUT_INPUT.to_string());
            changed = true;
        }
    }
    changed
}

/// Keeps a scene project's character row in step with its assets. Without a
/// character asset, the untouched template row (older projects all got one)
/// is dropped, so no phantom `character.glb` shows; with one and no locked
/// character row yet, the row is added for the character's export. Rows the
/// user changed are left alone. Returns true when `def` changed.
pub fn fit_presets(def: &mut PipelineDef, manifest: &ProjectManifest) -> bool {
    if !matches!(
        manifest.template,
        ProjectTemplate::FloatingDiorama | ProjectTemplate::Room
    ) {
        return false;
    }
    let character = manifest
        .assets
        .iter()
        .find(|a| a.kind == crate::AssetKind::Character);
    let default = character_preset();
    match character {
        None => {
            let before = def.presets.len();
            def.presets.retain(|p| *p != default);
            def.presets.len() != before
        }
        Some(asset) => {
            // The export is named after the model file it was built from.
            let stem = |file: &str| {
                Path::new(file)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            };
            let model = asset
                .latest()
                .and_then(|v| stem(&v.file))
                .unwrap_or_else(|| default.model.clone());
            let locked = def
                .presets
                .iter_mut()
                .find(|p| p.lock.as_deref() == Some(CHARACTER_LOCK));
            match locked {
                // A newer version of the character: the row follows its export.
                Some(row)
                    if row.model != model
                        && asset
                            .versions
                            .iter()
                            .any(|v| stem(&v.file).as_deref() == Some(row.model.as_str())) =>
                {
                    row.model = model;
                    true
                }
                Some(_) => false,
                None => {
                    def.presets.push(CompressPreset {
                        model,
                        label: asset.name.clone(),
                        ..default
                    });
                    true
                }
            }
        }
    }
}

/// The pipeline a project starts with: its template, or for an imported
/// project the proposal from its source folder (empty if the folder is gone).
pub fn initial(manifest: &ProjectManifest) -> PipelineDef {
    match (manifest.template, manifest.source_folder.as_deref()) {
        (ProjectTemplate::Imported, Some(source)) => match scan_import(Path::new(source)) {
            Ok(scan) => propose(&scan, Path::new(source)).pipeline,
            Err(err) => {
                tracing::warn!(error = %err, "import folder could not be scanned");
                for_template(ProjectTemplate::Imported)
            }
        },
        (template, _) => for_template(template),
    }
}

#[cfg(test)]
mod tests;
