//! The proposal built from the versioned scripts of an imported folder:
//! each script is classified by role and subject, and the newest version
//! of each becomes a step.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::s;
use crate::pipeline::{PIPELINE_VERSION, PipelineDef, StepDef, StepKind};
use crate::project::{BlendFile, ImportScan};

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
