//! Where models come back: linking inbox files to their sends, By hand
//! models handed back, accepted models, and copying records between projects.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::send::next_id;
use super::{
    Entry, InboxLink, JobState, ProviderJob, ProviderJobs, Stored, Workspace, persist, send_notes,
    touch_asset, update,
};
use crate::assets::{Asset, AssetKind, AssetStatus};
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::inbox::{self, InboxItem};
use crate::pipeline::history::now_ms;
use crate::providers::handback::{self, DownloadsScan};
use crate::providers::http::StopFlag;
use crate::providers::presets::PresetVariant;

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
                let id = next_id(&jobs, &dir, to, &copy.id);
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
        let config = self.shared.workspace().map(|w| w.config);
        {
            let jobs = self.shared.jobs();
            for item in items.iter_mut() {
                let stem = file_stem(&item.id);
                let Some(id) = match_job(&stem, jobs.iter().map(|e| e.stored.job.id.as_str()))
                else {
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
                if let Some(config) = &config {
                    let kind = match job.variant {
                        PresetVariant::Character => AssetKind::Character,
                        PresetVariant::Prop => AssetKind::Prop,
                    };
                    inbox::apply_send(item, kind, send_notes(job), config);
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

    /// Marks the job behind an accepted inbox model as done.
    pub fn accepted(&self, item: &str, version: u32) {
        let id = {
            let jobs = self.shared.jobs();
            let by_file = jobs
                .iter()
                .find(|e| e.stored.job.inbox_file.as_deref() == Some(item))
                .map(|e| e.stored.job.id.clone());
            by_file.or_else(|| {
                match_job(
                    &file_stem(item),
                    jobs.iter().map(|e| e.stored.job.id.as_str()),
                )
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
                Ok(())
            });
        }
    }
}

pub(super) fn write_inbox(workspace: &Workspace, id: &str, bytes: &[u8]) -> CoreResult<String> {
    let dir = inbox::inbox_dir(&workspace.root, &workspace.config);
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
