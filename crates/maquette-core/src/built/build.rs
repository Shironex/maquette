//! Building a built asset: its files are copied into a scratch folder under
//! the workspace cache, Blender runs [`super::BUILD_SCRIPT`] there headless
//! on the runner's own lane, and the GLB becomes the asset's next version.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use super::blender::{self, Ran, work_dir};
use super::{BuildRecord, file_hash, key, load, update};
use crate::assets::{self, Asset, AssetStatus, AssetVersion, Stage};
use crate::error::{CoreError, CoreResult};
use crate::inbox::checks::{self, CheckStatus, SendNotes};
use crate::inbox::{analyse, stats_of};
use crate::pipeline::runner::{NullSink, PipelineRunner};
use crate::pipeline::tools::Toolbox;
use crate::project::{self, ProjectManifest, now_rfc3339};
use crate::toolchain::ToolId;
use crate::workspace::WorkspaceConfig;

/// Checks that say nothing about a model made on purpose by a script:
/// separate objects are its pivots, joined parts and plain materials its
/// choice.
const SKIPPED_CHECKS: [&str; 3] = ["objects", "merged", "pbr"];

/// What a build did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BuildResult {
    /// A new version.
    Built,
    /// Blender ran and made the newest version's exact bytes again.
    Unchanged,
    /// The files match the last good build; Blender did not run.
    UpToDate,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuildOutcome {
    pub result: BuildResult,
    pub asset: Asset,
    /// The project after the build, so the Board can update.
    pub project: ProjectManifest,
    /// The version it made, with `built`.
    pub version: Option<AssetVersion>,
    pub record: Option<BuildRecord>,
    /// The end of Blender's log when the build failed.
    pub log: Vec<String>,
}

/// The project and tools a build runs with.
pub struct BuildContext<'a> {
    pub workspace_root: &'a Path,
    pub config: &'a WorkspaceConfig,
    pub project: &'a str,
    pub tools: &'a Toolbox,
}

/// The build service. Cheap to clone; builds run one at a time.
#[derive(Clone)]
pub struct Builder {
    runner: PipelineRunner,
    one: Arc<Mutex<()>>,
    /// `project/asset` of the builds asked for and not done yet.
    busy: Arc<Mutex<BTreeSet<String>>>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            runner: PipelineRunner::new(Arc::new(NullSink)),
            one: Arc::new(Mutex::new(())),
            busy: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }
}

/// Takes a build off the busy list when it ends, however it ends.
struct Busy<'a> {
    set: &'a Mutex<BTreeSet<String>>,
    key: String,
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.set
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.key);
    }
}

/// The titles of the checks a built model did not pass.
/// The checks a new version gets that it did not pass, and its stats.
pub(crate) fn warnings(
    output: &Path,
    asset: &Asset,
    config: &WorkspaceConfig,
) -> (Vec<String>, Option<crate::assets::ModelStats>) {
    let Ok(report) = analyse(output) else {
        return (Vec::new(), None);
    };
    let found = checks::run(&report, asset.kind, config, SendNotes::default(), None)
        .into_iter()
        .filter(|c| c.status != CheckStatus::Pass && !SKIPPED_CHECKS.contains(&c.id.as_str()))
        .map(|c| c.title)
        .collect();
    (found, Some(stats_of(&report)))
}

impl Builder {
    /// The assets of `project` building now or waiting their turn.
    pub fn busy(&self, project: &str) -> Vec<String> {
        let prefix = format!("{project}/");
        self.busy
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(|k| k.strip_prefix(&prefix).map(str::to_string))
            .collect()
    }

    /// Builds `asset_id` of `ctx.project`: nothing happens when its files
    /// match the last good build, unless `force`. `notify` hears the
    /// project while the asset is building and once it is done. Blocks
    /// until Blender is done.
    ///
    /// # Errors
    ///
    /// Fails for an asset that is not built, a missing file, without
    /// Blender, or when the version cannot be written. A script that fails
    /// is not an error: the outcome says `failed` with its log.
    pub fn build(
        &self,
        ctx: &BuildContext<'_>,
        asset_id: &str,
        force: bool,
        notify: &dyn Fn(&ProjectManifest),
    ) -> CoreResult<BuildOutcome> {
        let layout = &ctx.config.layout;
        let project_dir = project::dir(ctx.workspace_root, layout, ctx.project)?;
        let manifest = project::load(ctx.workspace_root, layout, ctx.project)?;
        let asset = assets::find(&manifest, asset_id)?.clone();
        let source = load(&project_dir)?
            .assets
            .get(&asset.id)
            .cloned()
            .ok_or_else(|| CoreError::Invalid(format!("{} is not a built asset.", asset.name)))?;
        let key = key(&project_dir, &source).map_err(|missing| {
            CoreError::Invalid(format!(
                "{} cannot build: {} missing from the project.",
                asset.name,
                missing.join(", ")
            ))
        })?;
        let current = source
            .last
            .as_ref()
            .is_some_and(|l| l.key == key && l.error.is_none())
            && asset.latest().is_some();
        if current && !force {
            return Ok(BuildOutcome {
                result: BuildResult::UpToDate,
                asset,
                project: manifest,
                version: None,
                record: source.last,
                log: Vec::new(),
            });
        }
        if !ctx.tools.has(ToolId::Blender) {
            return Err(CoreError::Unavailable(
                "Blender is not installed, so the built asset cannot build. Install it, then try again."
                    .to_string(),
            ));
        }
        let key_busy = format!("{}/{}", ctx.project, asset.id);
        self.busy
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key_busy.clone());
        let _busy = Busy {
            set: &self.busy,
            key: key_busy,
        };
        let _one = self.one.lock().unwrap_or_else(PoisonError::into_inner);
        let work = work_dir(ctx.workspace_root, ctx.config, ctx.project, &asset.id);
        let status = asset.status;
        notify(&project::modify(
            ctx.workspace_root,
            layout,
            ctx.project,
            |m| assets::set_status(m, &asset.id, AssetStatus::Running),
        )?);
        let ran = blender::stage_files(&project_dir, &work, &source)
            .and_then(|()| blender::run(&self.runner, &work, &source, ctx.tools));
        let outcome = match ran {
            Ok(ran) => self.record(ctx, &asset, &key, ran),
            Err(err) => Err(err),
        };
        if outcome.is_err() {
            // Leave the card as it was rather than spinning forever.
            if let Ok(m) = project::modify(ctx.workspace_root, layout, ctx.project, |m| {
                assets::set_status(m, &asset.id, status)
            }) {
                notify(&m);
            }
        }
        if let Ok(done) = &outcome {
            notify(&done.project);
        }
        outcome
    }

    fn record(
        &self,
        ctx: &BuildContext<'_>,
        asset: &Asset,
        key: &str,
        ran: Ran,
    ) -> CoreResult<BuildOutcome> {
        let layout = &ctx.config.layout;
        let project_dir = project::dir(ctx.workspace_root, layout, ctx.project)?;
        let (output, report) = match ran {
            Ran::Failed { error, log } => {
                let record = BuildRecord {
                    key: key.to_string(),
                    at: now_rfc3339(),
                    version: None,
                    error: Some(error),
                    objects: Vec::new(),
                    materials: Vec::new(),
                    triangles: 0,
                    warnings: Vec::new(),
                };
                save_last(&project_dir, &asset.id, &record)?;
                let manifest = project::modify(ctx.workspace_root, layout, ctx.project, |m| {
                    assets::set_status(m, &asset.id, AssetStatus::Failed)
                })?;
                return Ok(BuildOutcome {
                    result: BuildResult::Failed,
                    asset: assets::find(&manifest, &asset.id)?.clone(),
                    project: manifest,
                    version: None,
                    record: Some(record),
                    log,
                });
            }
            Ran::Made { output, report } => (output, report),
        };
        let (warnings, stats) = warnings(&output, asset, ctx.config);
        let hash = file_hash(&output);
        let mut made: Option<AssetVersion> = None;
        let mut matched: Option<u32> = None;
        let manifest = project::modify(ctx.workspace_root, layout, ctx.project, |m| {
            let latest = assets::find(m, &asset.id)?.latest().cloned();
            let same = latest
                .as_ref()
                .filter(|v| hash.is_some() && file_hash(&project_dir.join(&v.file)) == hash);
            if let Some(v) = same {
                matched = Some(v.version);
            } else {
                made = Some(add_version(
                    m,
                    &project_dir,
                    &asset.id,
                    &output,
                    stats.clone(),
                )?);
            }
            if assets::find(m, &asset.id)?.stage < Stage::Blender {
                assets::move_to(m, &asset.id, Stage::Blender)?;
            }
            assets::set_status(m, &asset.id, AssetStatus::Done)
        })?;
        let record = BuildRecord {
            key: key.to_string(),
            at: now_rfc3339(),
            version: made.as_ref().map(|v| v.version).or(matched),
            error: None,
            objects: report.objects,
            materials: report.materials,
            triangles: report.triangles,
            warnings,
        };
        save_last(&project_dir, &asset.id, &record)?;
        Ok(BuildOutcome {
            result: if made.is_some() {
                BuildResult::Built
            } else {
                BuildResult::Unchanged
            },
            asset: assets::find(&manifest, &asset.id)?.clone(),
            project: manifest,
            version: made,
            record: Some(record),
            log: Vec::new(),
        })
    }
}

/// Copies the GLB in as `models/<id>_v<n>.glb` and records the version.
pub(crate) fn add_version(
    manifest: &mut ProjectManifest,
    project_dir: &Path,
    id: &str,
    output: &Path,
    stats: Option<crate::assets::ModelStats>,
) -> CoreResult<AssetVersion> {
    let number = assets::find(manifest, id)?.next_version();
    let file_name = format!("{}_v{number}.glb", id.replace('-', "_"));
    let models = project_dir.join("models");
    let target = models.join(&file_name);
    if target.exists() {
        return Err(CoreError::Invalid(format!(
            "models/{file_name} already exists. Rename it or remove it first."
        )));
    }
    fs::create_dir_all(&models).map_err(|e| CoreError::io(&models, e))?;
    fs::copy(output, &target).map_err(|e| CoreError::io(&target, e))?;
    assets::add_version(manifest, id, format!("models/{file_name}"), stats)
}

fn save_last(project_dir: &Path, id: &str, record: &BuildRecord) -> CoreResult<()> {
    update(project_dir, |file| {
        if let Some(source) = file.assets.get_mut(id) {
            source.last = Some(record.clone());
        }
        Ok(())
    })
}
