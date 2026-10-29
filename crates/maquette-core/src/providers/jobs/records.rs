//! Where models come back: linking inbox files to their sends, By hand
//! models handed back, accepted models, and copying records between projects.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::ids::next_id;
use super::landed;
use super::{
    Entry, InboxLink, JobState, Provider, ProviderJob, ProviderJobs, Stored, Workspace, persist,
    send_notes, touch_asset, update,
};
use crate::assets::{Asset, AssetKind, AssetStatus};
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::inbox::{self, InboxItem};
use crate::pipeline::history::now_ms;
use crate::project;
use crate::providers::handback::{self, DownloadsScan};
use crate::providers::http::StopFlag;
use crate::providers::presets::PresetVariant;
use crate::settings::game::{self, FolderBudget};

/// Lower case with `_` and spaces read as `-`, for matching file names to
/// job ids.
fn normalize(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == '_' || c == ' ' { '-' } else { c })
        .collect()
}

/// The job id an inbox file name points at: the whole stem, or the
/// longest job id it starts with, followed by `-`.
pub(super) fn match_job<'a>(stem: &str, ids: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let stem = normalize(stem);
    ids.filter(|id| {
        let id = normalize(id);
        stem == id
            || stem
                .strip_prefix(&id)
                .is_some_and(|rest| rest.starts_with('-'))
    })
    .max_by_key(|id| id.len())
}

/// The job a model file is for: its export name (`maq_<asset>_v<n>_<provider>`)
/// first, else [`match_job`] on its stem.
pub(super) fn job_for_file<'a>(file: &str, ids: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let ids: Vec<&'a str> = ids.collect();
    if let Some(named) = crate::providers::naming::parse(file) {
        let id = named.job_id();
        if let Some(found) = ids.iter().copied().find(|i| *i == id) {
            return Some(found);
        }
    }
    match_job(&file_stem(file), ids.into_iter())
}

fn file_stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned())
}

impl ProviderJobs {
    /// Drops finished jobs of a project from the queue.
    pub fn clear_finished(&self, project: &str) {
        self.shared.jobs().retain(|e| {
            e.stored.job.project != project
                || e.stored.job.state.is_active()
                || e.stored.job.state == JobState::Inbox
        });
        persist(&self.shared, project);
    }

    /// Copies the finished send records of `asset` in project `from` to the
    /// copy `copy` in project `to`, so its history comes along. Jobs still
    /// running or waiting in the inbox stay where they are. `refs` maps old
    /// image paths to where Copy to project put them. Returns how many.
    pub fn copy_records(
        &self,
        from: &str,
        asset: &str,
        to: &str,
        copy: &Asset,
        refs: &[(String, String)],
    ) -> usize {
        let Some(workspace) = self.shared.workspace() else {
            return 0;
        };
        let Ok(dir) = workspace.project_dir(to) else {
            return 0;
        };
        let _one = self.shared.sending();
        let copied = {
            let mut jobs = self.shared.jobs();
            let sources: Vec<Stored> = jobs
                .iter()
                .filter(|e| e.stored.job.project == from && e.stored.job.asset == asset)
                .filter(|e| {
                    !e.stored.job.state.is_active() && e.stored.job.state != JobState::Inbox
                })
                .map(|e| e.stored.clone())
                .collect();
            for source in &sources {
                let id = next_id(&jobs, &dir, &copy.id);
                let mut job = source.job.clone();
                job.id = id;
                job.project = to.to_string();
                job.asset.clone_from(&copy.id);
                job.asset_name.clone_from(&copy.name);
                job.batch = None;
                job.images = job
                    .images
                    .iter()
                    .map(|image| {
                        refs.iter()
                            .find(|(old, _)| old == image)
                            .map_or_else(|| image.clone(), |(_, new)| new.clone())
                    })
                    .collect();
                jobs.push(Entry {
                    stored: Stored {
                        job,
                        chain: source.chain.clone(),
                    },
                    stop: Arc::new(StopFlag::default()),
                    thread: false,
                    cancelled: false,
                });
            }
            sources.len()
        };
        if copied > 0 {
            persist(&self.shared, to);
        }
        copied
    }

    /// Fills `link` on inbox items that came from a send, and moves a By
    /// hand job whose model showed up in the inbox to the Inbox state.
    pub fn link_inbox(&self, items: &mut [InboxItem]) {
        let mut arrived = Vec::new();
        let workspace = self.shared.workspace();
        // A game project's folder budget for each asset, read once per call.
        let mut budgets: HashMap<(String, String), Option<FolderBudget>> = HashMap::new();
        {
            let jobs = self.shared.jobs();
            for item in items.iter_mut() {
                // The file a job saved first, then its name.
                let saved = jobs
                    .iter()
                    .find(|e| e.stored.job.inbox_file.as_deref() == Some(item.id.as_str()))
                    .map(|e| e.stored.job.id.as_str());
                let Some(id) = saved.or_else(|| {
                    job_for_file(&item.id, jobs.iter().map(|e| e.stored.job.id.as_str()))
                }) else {
                    continue;
                };
                let Some(entry) = jobs.iter().find(|e| e.stored.job.id == id) else {
                    continue;
                };
                let job = &entry.stored.job;
                item.link = Some(InboxLink {
                    job: job.id.clone(),
                    project: job.project.clone(),
                    asset: job.asset.clone(),
                    asset_name: job.asset_name.clone(),
                    provider: job.provider,
                    redo_of: job.redo_of.clone(),
                });
                if let Some(workspace) = &workspace {
                    let kind = match job.variant {
                        PresetVariant::Character => AssetKind::Character,
                        PresetVariant::Prop => AssetKind::Prop,
                    };
                    let folder = budgets
                        .entry((job.project.clone(), job.asset.clone()))
                        .or_insert_with(|| {
                            project::load(&workspace.root, &workspace.config.layout, &job.project)
                                .ok()
                                .and_then(|m| game::asset_budget(&m, &job.asset))
                        });
                    inbox::apply_send(
                        item,
                        kind,
                        send_notes(job),
                        &workspace.config,
                        folder.as_ref(),
                    );
                }
                if job.state == JobState::Waiting {
                    arrived.push((job.id.clone(), item.id.clone()));
                }
            }
        }
        for (job, file) in arrived {
            let _ = self.arrived(&job, &file);
        }
    }

    fn arrived(&self, id: &str, file: &str) -> CoreResult<()> {
        if let Some(job) = self.get(id) {
            touch_asset(
                &self.shared,
                &job.project,
                &job.asset,
                None,
                AssetStatus::Review,
            );
        }
        update(&self.shared, id, true, |stored| {
            stored.job.state = JobState::Inbox;
            stored.job.progress = 1.0;
            stored.job.finished_at_ms = Some(now_ms());
            stored.job.inbox_file = Some(file.to_string());
            stored.job.message = Some("In the Inbox, ready for its checks".to_string());
            Ok(())
        })
    }

    /// Looks in `downloads` for models that came down after the project's
    /// By hand sends were written, and offers a waiting send for each.
    /// Reads only; nothing is copied until a match is confirmed with
    /// [`ProviderJobs::hand_back`].
    pub fn check_downloads(&self, project: &str, downloads: &Path) -> DownloadsScan {
        handback::check_downloads(downloads, &self.list(Some(project)))
    }

    /// By hand: copies a GLB someone dropped on the asset's card (or the
    /// one GLB inside a zip) into the inbox under the job's name, so it goes
    /// through the usual checks. The original file is left alone.
    pub fn hand_back(&self, id: &str, file: &Path) -> CoreResult<ProviderJob> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let job = self
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))?;
        // Read, never moved: the original stays where it was.
        let model = handback::read_model(file)?;
        let name = write_inbox(&workspace, &job.id, &model.bytes)?;
        if job.variant == PresetVariant::Character {
            let _ = inbox::set_kind(
                &workspace.root,
                &workspace.config,
                &name,
                AssetKind::Character,
            );
        }
        self.arrived(&job.id, &name)?;
        self.get(id)
            .ok_or_else(|| CoreError::NotFound(format!("the job {id}")))
    }

    /// Marks the job behind an accepted inbox model as done, and closes
    /// every other By hand send of `asset` still waiting for its model:
    /// the asset has one now. Returns how many of those closed.
    pub fn accepted(&self, item: &str, project: &str, asset: &str, version: u32) -> usize {
        let id = {
            let jobs = self.shared.jobs();
            let by_file = jobs
                .iter()
                .find(|e| e.stored.job.inbox_file.as_deref() == Some(item))
                .map(|e| e.stored.job.id.clone());
            by_file.or_else(|| {
                job_for_file(item, jobs.iter().map(|e| e.stored.job.id.as_str()))
                    .map(str::to_string)
            })
        };
        if let Some(id) = id {
            let _ = update(&self.shared, &id, true, |stored| {
                stored.job.state = JobState::Accepted;
                stored.job.version = Some(version);
                stored.job.inbox_file = Some(item.to_string());
                stored.job.message = Some(format!("Accepted as v{version}"));
                stored.job.finished_at_ms.get_or_insert_with(now_ms);
                super::split_bake::on_accept(stored, version);
                Ok(())
            });
        }
        landed::close_waits(&self.shared, project, asset)
    }

    /// Closes the send behind an inbox model that was discarded: a job
    /// still in the Inbox state ends as cancelled with a note, since its
    /// model is gone. Its ledger line was written when the model arrived, so
    /// nothing is spent or refunded by this. Returns the job's id.
    pub fn discarded(&self, item: &str) -> Option<String> {
        let id = {
            let jobs = self.shared.jobs();
            jobs.iter()
                .find(|e| e.stored.job.inbox_file.as_deref() == Some(item))
                .map(|e| e.stored.job.id.clone())
                .or_else(|| {
                    job_for_file(item, jobs.iter().map(|e| e.stored.job.id.as_str()))
                        .map(str::to_string)
                })
        }?;
        let mut closed = false;
        let _ = update(&self.shared, &id, true, |stored| {
            if stored.job.state == JobState::Inbox {
                stored.job.state = JobState::Cancelled;
                stored.job.inbox_file = None;
                stored.job.message = Some("Discarded from the Inbox".to_string());
                stored.job.finished_at_ms.get_or_insert_with(now_ms);
                closed = true;
            }
            Ok(())
        });
        closed.then_some(id)
    }

    /// Brings a model someone picked an asset for (a download the watcher
    /// could not place) into the inbox for `asset` of `project`, as an
    /// exact name match does: for the asset's newest waiting By hand send
    /// when it has one, else aimed at the asset, so Accept goes there. The
    /// usual checks run there, and accepting it closes the asset's sends
    /// still waiting. The original file is left alone.
    pub fn attach(&self, project: &str, asset: &str, file: &Path) -> CoreResult<ModelAttached> {
        let workspace = self
            .shared
            .workspace()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let manifest = project::load(&workspace.root, &workspace.config.layout, project)?;
        let found = crate::assets::find(&manifest, asset)?;
        let (asset, asset_name) = (found.id.clone(), found.name.clone());
        let mut waiting: Vec<ProviderJob> = self
            .list(Some(project))
            .into_iter()
            .filter(|j| j.asset == asset)
            .filter(|j| j.provider == Provider::ByHand && j.state == JobState::Waiting)
            .collect();
        waiting.sort_by(|a, b| b.created_at_ms.total_cmp(&a.created_at_ms));
        let (job, inbox_file) = match waiting.first() {
            Some(job) => {
                let back = self.hand_back(&job.id, file)?;
                let item = back.inbox_file.ok_or_else(|| {
                    CoreError::Invalid(format!("{} did not reach the inbox.", job.id))
                })?;
                (Some(job.id.clone()), item)
            }
            None => {
                let model = handback::read_model(file)?;
                let item = write_inbox(&workspace, &format!("download-{asset}"), &model.bytes)?;
                let aim = crate::sends::InboxAim {
                    project: project.to_string(),
                    asset: asset.clone(),
                    asset_name: asset_name.clone(),
                };
                inbox::set_aim(&workspace.root, &workspace.config, &item, Some(aim))?;
                (None, item)
            }
        };
        Ok(ModelAttached {
            job,
            inbox_file,
            asset,
            asset_name,
        })
    }

    /// Closes the By hand sends of `project` still waiting while their
    /// asset already has a newer version, for a model that landed with no
    /// send behind it (the library). That model was not made on the site,
    /// so the credits ledger is left alone. Returns how many closed.
    pub fn settle_handed(&self, project: &str) -> usize {
        landed::close_outdated(&self.shared, project, landed::Spend::None)
    }
}

/// What [`ProviderJobs::attach`] did.
#[derive(Debug)]
pub struct ModelAttached {
    /// The By hand send it came in for, when the asset had one waiting.
    pub job: Option<String>,
    /// The inbox file it became.
    pub inbox_file: String,
    pub asset: String,
    pub asset_name: String,
}

pub(super) fn write_inbox(workspace: &Workspace, id: &str, bytes: &[u8]) -> CoreResult<String> {
    let dir = crate::sends::inbox_dir(&workspace.root, &workspace.config);
    fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
    let name = (1u32..)
        .map(|n| {
            if n == 1 {
                format!("{id}.glb")
            } else {
                format!("{id}-{n}.glb")
            }
        })
        .find(|name| !dir.join(name).exists())
        .unwrap_or_else(|| format!("{id}.glb"));
    let path = dir.join(&name);
    write_atomic(&path, bytes).map_err(|e| CoreError::io(&path, e))?;
    Ok(name)
}
