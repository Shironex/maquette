//! Starting pipelines: one per project template, and a proposal built from
//! the versioned scripts of an imported folder.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CompressPreset, CompressSpec, PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::project::{BlendFile, ImportScan, ProjectManifest, ProjectTemplate, scan_import};
use crate::workspace::{MeshoptLevel, TextureCodec};

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
            &["blender/scene.blend", "models/*.glb", "models/*.gltf"],
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

/// What an import found and how it was turned into steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportProposal {
    pub pipeline: PipelineDef,
    /// Scripts that did not map to a step (inspect, probe and preview
    /// helpers, or older versions).
    pub unused: Vec<String>,
}

/// A tool script split into its role, subject and version, for example
/// `export_island_web_v7.py` is (export, island, 7).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Script {
    file: String,
    role: Role,
    subject: String,
    version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    Build,
    Bake,
    Export,
    Compress,
    Posters,
    Sheet,
}

fn classify(file: &str) -> Option<Script> {
    let parsed = BlendFile::parse(file);
    let stem = parsed.stem.to_ascii_lowercase();
    let ext = file
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let (role, rest) = if let Some(rest) = stem.strip_prefix("build_") {
        (Role::Build, rest)
    } else if let Some(rest) = stem.strip_prefix("bake_") {
        (Role::Bake, rest)
    } else if let Some(rest) = stem.strip_prefix("export_") {
        (Role::Export, rest)
    } else if let Some(rest) = stem.strip_prefix("compress_") {
        (Role::Compress, rest)
    } else if stem == "render_stops" || stem.starts_with("capture_") || stem.starts_with("render_")
    {
        (Role::Posters, stem.as_str())
    } else if stem.starts_with("contact_sheet") {
        (Role::Sheet, stem.as_str())
    } else {
        return None;
    };
    let python = ext == "py";
    if matches!(
        role,
        Role::Build | Role::Bake | Role::Export | Role::Posters
    ) && !python
    {
        return None;
    }
    let subject = rest
        .trim_end_matches("_light")
        .trim_end_matches("_web")
        .to_string();
    Some(Script {
        file: file.to_string(),
        role,
        subject,
        version: parsed.version.unwrap_or(0),
    })
}

/// A versioned blend, with `_baked` peeled off before the version.
#[derive(Debug, Clone)]
struct Blend {
    file: String,
    stem: String,
    version: u32,
    baked: bool,
}

fn blends(scan: &ImportScan) -> Vec<Blend> {
    scan.blend_files
        .iter()
        .map(|b| {
            let name = b.file.rsplit('/').next().unwrap_or(&b.file);
            let base = name.strip_suffix(".blend").unwrap_or(name);
            let (base, baked) = match base.strip_suffix("_baked") {
                Some(base) => (base, true),
                None => (base, false),
            };
            let parsed = BlendFile::parse(&format!("{base}.blend"));
            Blend {
                file: b.file.clone(),
                stem: parsed.stem.to_ascii_lowercase(),
                version: parsed.version.unwrap_or(0),
                baked,
            }
        })
        .collect()
}

fn newest<'a>(list: impl Iterator<Item = &'a Blend>) -> Option<&'a Blend> {
    list.max_by(|a, b| a.version.cmp(&b.version).then_with(|| b.file.cmp(&a.file)))
}

/// The blend a script works on: the newest one whose stem mentions the
/// script's subject, baked or not as asked. Falls back to any newest blend.
fn blend_for<'a>(all: &'a [Blend], subject: &str, baked: bool) -> Option<&'a Blend> {
    newest(
        all.iter()
            .filter(|b| b.baked == baked && b.stem.contains(subject)),
    )
    .or_else(|| newest(all.iter().filter(|b| b.baked == baked)))
}

fn dir_of(file: &str) -> &str {
    file.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

fn join(dir: &str, rest: &str) -> String {
    if dir.is_empty() {
        rest.to_string()
    } else {
        format!("{dir}/{rest}")
    }
}

/// Steps proposed from an imported folder's scripts. Only the newest
/// version of each script is used, in the order build, bake, export,
/// compress, posters, contact sheet, each depending on the step before.
/// Paths point at the folder's newest blends, so the proposal runs as found
/// and the user edits it from there. The pipeline runs inside `source`.
pub fn propose(scan: &ImportScan, source: &Path) -> ImportProposal {
    let mut latest: BTreeMap<(Role, String), Script> = BTreeMap::new();
    let mut unused = Vec::new();
    for file in &scan.tool_scripts {
        match classify(file) {
            Some(script) => {
                let key = (script.role, script.subject.clone());
                match latest.get(&key) {
                    Some(prev) if prev.version >= script.version => unused.push(file.clone()),
                    Some(prev) => {
                        unused.push(prev.file.clone());
                        latest.insert(key, script);
                    }
                    None => {
                        latest.insert(key, script);
                    }
                }
            }
            None => unused.push(file.clone()),
        }
    }
    unused.sort();

    let all = blends(scan);
    let mut steps: Vec<StepDef> = Vec::new();
    let mut prev: Option<String> = None;
    let mut baked_blend: Option<String> = None;
    let deps = |prev: &Option<String>| prev.iter().cloned().collect::<Vec<_>>();
    // Every export writes into one web folder named after the scene version
    // (`models/web_v7`), and the bake into `models/bake_v7`.
    let scene = latest
        .values()
        .find(|s| s.role == Role::Bake)
        .and_then(|s| blend_for(&all, &s.subject, false))
        .or_else(|| newest(all.iter().filter(|b| !b.baked)));
    let (web_dir, bake_dir) = match scene {
        Some(b) => (
            join(dir_of(&b.file), &format!("web_v{}", b.version)),
            join(dir_of(&b.file), &format!("bake_v{}", b.version)),
        ),
        None => ("web".to_string(), "bake".to_string()),
    };

    for script in latest.values() {
        let path = format!("tools/{}", script.file);
        let id = unique_id(
            &steps,
            &format!("{:?}-{}", script.role, script.subject).to_lowercase(),
        );
        let subject_name = title(&script.subject);
        let step = match script.role {
            Role::Build => {
                let Some(target) = blend_for(&all, &script.subject, false) else {
                    unused.push(script.file.clone());
                    continue;
                };
                let source_blend = all
                    .iter()
                    .filter(|b| !b.baked && b.stem == target.stem && b.version < target.version)
                    .max_by_key(|b| b.version)
                    .unwrap_or(target);
                let mut args = s(&[
                    "-b",
                    "--factory-startup",
                    "--python-exit-code",
                    "1",
                    &source_blend.file,
                    "--python",
                    "{script}",
                    "--",
                ]);
                args.extend(s(&["--out", &target.file]));
                StepDef {
                    id,
                    name: format!("Build {subject_name}"),
                    kind: StepKind::Blender,
                    command: None,
                    script: Some(path),
                    args,
                    inputs: vec![source_blend.file.clone()],
                    outputs: vec![target.file.clone()],
                    depends_on: deps(&prev),
                    compress: None,
                }
            }
            Role::Bake => {
                let Some(target) = blend_for(&all, &script.subject, false) else {
                    unused.push(script.file.clone());
                    continue;
                };
                let out_blend = target.file.replace(".blend", "_baked.blend");
                baked_blend = Some(out_blend.clone());
                let mut args = s(&[
                    "-b",
                    "--factory-startup",
                    "--python-exit-code",
                    "1",
                    &target.file,
                    "--python",
                    "{script}",
                    "--",
                ]);
                args.extend(s(&["--out-dir", &bake_dir, "--out-blend", &out_blend]));
                StepDef {
                    id,
                    name: "Bake light".into(),
                    kind: StepKind::Blender,
                    command: None,
                    script: Some(path),
                    args,
                    inputs: vec![target.file.clone()],
                    outputs: vec![format!("{bake_dir}/*.png"), out_blend],
                    depends_on: deps(&prev),
                    compress: None,
                }
            }
            Role::Export => {
                let own = newest(
                    all.iter()
                        .filter(|b| b.baked && b.stem.contains(&script.subject)),
                )
                .or_else(|| {
                    newest(
                        all.iter()
                            .filter(|b| !b.baked && b.stem.contains(&script.subject)),
                    )
                })
                .map(|b| b.file.clone());
                let Some(blend) = own.or_else(|| baked_blend.clone()) else {
                    unused.push(script.file.clone());
                    continue;
                };
                let mut args = s(&[
                    "-b",
                    "--factory-startup",
                    "--python-exit-code",
                    "1",
                    &blend,
                    "--python",
                    "{script}",
                    "--",
                ]);
                args.extend(s(&["--bake", &bake_dir, "--out", &web_dir]));
                StepDef {
                    id,
                    name: format!("Export {subject_name}"),
                    kind: StepKind::Blender,
                    command: None,
                    script: Some(path),
                    args,
                    inputs: vec![blend],
                    outputs: vec![format!("{web_dir}/raw/*.glb")],
                    depends_on: deps(&prev),
                    compress: None,
                }
            }
            Role::Compress => StepDef {
                id,
                name: "Compress".into(),
                kind: StepKind::Shell,
                command: Some("sh".into()),
                script: Some(path),
                args: s(&["{script}", &format!("{web_dir}/dist")]),
                inputs: vec![format!("{web_dir}/raw/*.glb")],
                outputs: vec![format!("{web_dir}/dist/*.glb")],
                depends_on: deps(&prev),
                compress: None,
            },
            Role::Posters => {
                let blend = baked_blend
                    .clone()
                    .or_else(|| newest(all.iter().filter(|b| b.baked)).map(|b| b.file.clone()))
                    .or_else(|| newest(all.iter()).map(|b| b.file.clone()));
                let Some(blend) = blend else {
                    unused.push(script.file.clone());
                    continue;
                };
                let mut args = s(&[
                    "-b",
                    "--factory-startup",
                    "--python-exit-code",
                    "1",
                    &blend,
                    "--python",
                    "{script}",
                    "--",
                ]);
                args.extend(s(&["--out", "review/stops"]));
                StepDef {
                    id,
                    name: "Capture posters".into(),
                    kind: StepKind::Blender,
                    command: None,
                    script: Some(path),
                    args,
                    inputs: vec![blend],
                    outputs: vec!["review/stops/*.png".into()],
                    depends_on: deps(&prev),
                    compress: None,
                }
            }
            Role::Sheet => StepDef {
                id,
                name: "Contact sheet".into(),
                kind: StepKind::Shell,
                command: Some("sh".into()),
                script: Some(path),
                args: s(&["{script}", "stops", "stops-sheet.jpg"]),
                inputs: vec!["review/stops/*.png".into()],
                outputs: vec!["review/stops-sheet.jpg".into()],
                depends_on: deps(&prev),
                compress: None,
            },
        };
        prev = Some(step.id.clone());
        steps.push(step);
    }
    unused.sort();
    unused.dedup();
    ImportProposal {
        pipeline: PipelineDef {
            version: PIPELINE_VERSION,
            root: Some(source.display().to_string()),
            steps,
            presets: Vec::new(),
        },
        unused,
    }
}

fn unique_id(steps: &[StepDef], base: &str) -> String {
    let base = base.replace('_', "-");
    if !steps.iter().any(|s| s.id == base) {
        return base;
    }
    (2u32..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !steps.iter().any(|s| &s.id == id))
        .unwrap_or(base)
}

/// The subject as words for a sentence case step name ("Build island").
fn title(subject: &str) -> String {
    subject.replace('_', " ").to_lowercase()
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
mod tests {
    use super::*;
    use crate::pipeline::graph::{PipelineIssue, validate};

    fn names(def: &PipelineDef) -> Vec<&str> {
        def.steps.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn floating_diorama_matches_the_island_flow() {
        let def = for_template(ProjectTemplate::FloatingDiorama);
        assert_eq!(
            names(&def),
            [
                "Build scene",
                "Bake light",
                "Export GLB",
                "Compress",
                "Capture posters",
                "Ship"
            ]
        );
        for step in def.steps.iter().filter(|s| s.kind == StepKind::Blender) {
            assert_eq!(step.args[0], "-b", "{}", step.id);
            assert!(step.args.contains(&"{script}".to_string()));
        }
        // A new project has no blend yet: the build opens none, and every
        // step after it opens the blend the step before it wrote.
        let build = def.step("build").expect("build");
        assert_eq!(
            build.args[..6],
            s(&[
                "-b",
                "--factory-startup",
                "--python-exit-code",
                "1",
                "--python",
                "{script}"
            ])
        );
        for (id, blend) in [
            ("bake", "blender/scene_built.blend"),
            ("export", "blender/scene_baked.blend"),
            ("posters", "blender/scene_baked.blend"),
        ] {
            assert_eq!(def.step(id).expect(id).args[4], blend, "{id}");
            assert!(
                def.steps
                    .iter()
                    .any(|s| s.outputs.iter().any(|o| o == blend))
            );
        }
        // Outputs every starter script always writes, even with nothing to do.
        assert_eq!(
            def.step("bake").expect("bake").outputs,
            s(&["blender/bake/bake.json", "blender/scene_baked.blend"])
        );
        assert_eq!(
            def.step("posters").expect("posters").outputs,
            s(&["web/posters/stops.json"])
        );
        let ship = def.step("ship").expect("ship");
        let site = ship
            .args
            .iter()
            .position(|a| a == "--site")
            .expect("--site");
        assert_eq!(ship.args[site + 1], "{siteRepo}");
        assert!(!ship.args.iter().any(|a| a.starts_with("{siteRepo}/")));
        let compress = def.step("compress").expect("compress");
        assert_eq!(compress.kind, StepKind::GltfTransform);
        assert!(compress.compress.is_some());
        assert!(def.presets.is_empty(), "no character, no character row");
        assert!(validate(&def).iter().all(|i| !i.is_error()));
    }

    #[test]
    fn a_scene_gets_the_character_row_only_with_a_character() {
        use crate::assets::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
        let asset = |name: &str, kind: AssetKind, file: &str| Asset {
            id: name.to_lowercase(),
            name: name.into(),
            kind,
            stage: Stage::Blender,
            status: AssetStatus::Review,
            versions: vec![AssetVersion {
                version: 1,
                file: file.into(),
                created_at: String::new(),
                stats: None,
            }],
            thumbnail: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let mut manifest = ProjectManifest {
            version: 1,
            id: "moon-island".into(),
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            site_repo: None,
            source_folder: None,
            created_at: String::new(),
            assets: vec![asset("clay", AssetKind::Prop, "models/clay_v1.glb")],
        };
        // An older project: the template row, though it has no character.
        let mut def = for_template(ProjectTemplate::FloatingDiorama);
        def.presets = vec![character_preset()];
        assert!(fit_presets(&mut def, &manifest));
        assert!(def.presets.is_empty());
        assert!(!fit_presets(&mut def, &manifest), "settled");

        // A row the user changed stays.
        let mut edited = def.clone();
        edited.presets = vec![CompressPreset {
            texture_size: 1024,
            ..character_preset()
        }];
        assert!(!fit_presets(&mut edited, &manifest));
        assert_eq!(edited.presets.len(), 1);

        // A character arrives: its export gets the locked row.
        manifest.assets.push(asset(
            "Lantern keeper",
            AssetKind::Character,
            "models/lantern_keeper_v2.glb",
        ));
        assert!(fit_presets(&mut def, &manifest));
        assert_eq!(def.presets.len(), 1);
        assert_eq!(def.presets[0].model, "lantern_keeper_v2");
        assert_eq!(def.presets[0].label, "Lantern keeper");
        assert_eq!(def.presets[0].lock.as_deref(), Some(CHARACTER_LOCK));
        assert!(!fit_presets(&mut def, &manifest));
        // Its v3 is exported as lantern_keeper_v3: the row follows.
        let keeper = manifest.assets.last_mut().expect("keeper");
        keeper.versions.push(AssetVersion {
            version: 3,
            file: "models/lantern_keeper_v3.glb".into(),
            created_at: String::new(),
            stats: None,
        });
        assert!(fit_presets(&mut def, &manifest));
        assert_eq!(def.presets[0].model, "lantern_keeper_v3");

        // A character showcase keeps its own row as it is.
        let mut showcase = for_template(ProjectTemplate::CharacterShowcase);
        manifest.template = ProjectTemplate::CharacterShowcase;
        assert!(!fit_presets(&mut showcase, &manifest));
        assert_eq!(showcase.presets, vec![character_preset()]);
    }

    #[test]
    fn character_showcase_builds_its_own_scene() {
        let def = for_template(ProjectTemplate::CharacterShowcase);
        assert_eq!(
            names(&def),
            [
                "Build character",
                "Export GLB",
                "Compress",
                "Capture posters",
                "Ship"
            ]
        );
        let ids: Vec<&str> = def.steps.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["build", "export", "compress", "posters", "ship"]);
        // A fresh project has only models: the build opens no blend and
        // reads the optional base and the models as inputs.
        let build = def.step("build").expect("build");
        assert_eq!(build.args[4], "--python");
        assert!(build.inputs.contains(&"models/*.glb".to_string()));
        assert_eq!(build.outputs, s(&["blender/character_built.blend"]));
        for id in ["export", "posters"] {
            assert_eq!(
                def.step(id).expect(id).args[4],
                "blender/character_built.blend",
                "{id}"
            );
        }
        assert_eq!(
            def.step("export").expect("export").depends_on,
            s(&["build"])
        );
        assert!(validate(&def).iter().all(|i| !i.is_error()));
    }

    fn scan_of(blends: &[&str], scripts: &[&str]) -> ImportScan {
        ImportScan {
            path: "/src/island".into(),
            blend_files: blends.iter().map(|b| BlendFile::parse(b)).collect(),
            tool_scripts: scripts.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn import_proposes_steps_from_the_newest_scripts() {
        let scan = scan_of(
            &[
                "models/moon_island_v5.blend",
                "models/moon_island_v5_baked.blend",
                "models/moon_island_v6.blend",
                "models/moon_island_v7.blend",
                "models/moon_island_v7_baked.blend",
                "models/character_web_v4.blend",
            ],
            &[
                "bake_island_light.py",
                "build_island.py",
                "build_island_v6.py",
                "build_island_v7.py",
                "compress_island_web.sh",
                "compress_island_web_v7.sh",
                "contact_sheet.sh",
                "export_character_web.py",
                "export_character_web_v2.py",
                "export_island_web.py",
                "inspect_blend.py",
                "preview_models.py",
                "render_stops.py",
            ],
        );
        let proposal = propose(&scan, Path::new("/src/island"));
        let def = &proposal.pipeline;
        assert_eq!(def.root.as_deref(), Some("/src/island"));
        assert_eq!(
            names(def),
            [
                "Build island",
                "Bake light",
                "Export character",
                "Export island",
                "Compress",
                "Capture posters",
                "Contact sheet"
            ]
        );
        let build = &def.steps[0];
        assert_eq!(build.script.as_deref(), Some("tools/build_island_v7.py"));
        assert_eq!(
            build.inputs,
            vec!["models/moon_island_v6.blend".to_string()]
        );
        assert_eq!(
            build.outputs,
            vec!["models/moon_island_v7.blend".to_string()]
        );
        let bake = &def.steps[1];
        assert!(bake.args.contains(&"models/bake_v7".to_string()));
        assert!(
            bake.outputs
                .contains(&"models/moon_island_v7_baked.blend".to_string())
        );
        let character = &def.steps[2];
        assert_eq!(
            character.inputs,
            vec!["models/character_web_v4.blend".to_string()]
        );
        assert!(character.args.contains(&"models/web_v7".to_string()));
        let export = &def.steps[3];
        assert_eq!(
            export.inputs,
            vec!["models/moon_island_v7_baked.blend".to_string()]
        );
        assert!(export.args.contains(&"models/web_v7".to_string()));
        let compress = &def.steps[4];
        assert_eq!(compress.kind, StepKind::Shell);
        assert_eq!(
            compress.script.as_deref(),
            Some("tools/compress_island_web_v7.sh")
        );
        assert_eq!(
            compress.args,
            vec!["{script}".to_string(), "models/web_v7/dist".to_string()]
        );
        for pair in def.steps.windows(2) {
            assert_eq!(pair[1].depends_on, vec![pair[0].id.clone()]);
        }
        assert_eq!(
            proposal.unused,
            [
                "build_island.py",
                "build_island_v6.py",
                "compress_island_web.sh",
                "export_character_web.py",
                "inspect_blend.py",
                "preview_models.py"
            ]
        );
        let errors: Vec<PipelineIssue> =
            validate(def).into_iter().filter(|i| i.is_error()).collect();
        assert!(errors.is_empty(), "{errors:?}");
        for step in def.steps.iter().filter(|s| s.kind == StepKind::Blender) {
            assert_eq!(step.args[0], "-b");
        }
    }

    #[test]
    fn import_without_blends_proposes_nothing_blender() {
        let proposal = propose(
            &scan_of(&[], &["build_room.py", "notes.sh"]),
            Path::new("/x"),
        );
        assert!(proposal.pipeline.steps.is_empty());
        assert_eq!(proposal.unused, ["build_room.py", "notes.sh"]);
    }
}
