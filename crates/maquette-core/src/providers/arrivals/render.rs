//! A download's front picture: rendered headless from a GLB copy of it, on
//! a plain ground, one download at a time on a lane of its own, and cached
//! by the download's content hash in `<workspace cache>/download-pictures`.
//! The watcher only queues the render, so a slow Blender never holds up
//! the next file. Once the picture is in, every waiting send is ranked by
//! it ([`super::picture`]) and the download's waiters hear of it again.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::picture::{self, looks_like, percent, rank};
use super::{Arrival, ArrivalState, Arrivals, PictureState, is_fbx, is_open, suggest, waiting};
use crate::base::ports::PictureRender;
use crate::error::{CoreError, CoreResult};

/// The workspace cache folder of the pictures.
pub const PICTURES_DIR: &str = "download-pictures";

impl Arrivals {
    /// Lets the watcher render downloads to match them by picture. Without
    /// it (or without Blender) the name alone ranks the sends.
    pub fn set_renderer(&self, renderer: impl PictureRender + 'static) {
        self.inner
            .pictures
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .renderer = Some(Arc::new(renderer));
    }

    fn renderer(&self) -> Option<Arc<dyn PictureRender>> {
        self.inner
            .pictures
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .renderer
            .clone()
    }

    /// The project folder of `project` in the open workspace.
    fn project_dir(&self, project: &str) -> Option<PathBuf> {
        let (root, config) = self.inner.studio.workspace_paths()?;
        crate::project::dir(&root, &config.layout, project).ok()
    }

    /// Ranks every waiting send by name for a download that waits, and
    /// queues its picture when one can be made.
    pub(super) fn ranked_by_name(
        &self,
        mut arrival: Arrival,
        waiting: &[crate::providers::jobs::ProviderJob],
    ) -> Arrival {
        let dirs = |p: &str| self.project_dir(p);
        arrival.suggestions = rank(
            &arrival.name,
            arrival.inner.as_deref(),
            None,
            waiting,
            &dirs,
        );
        let can_render = !is_fbx(Path::new(&arrival.path)) && self.renderer().is_some();
        arrival.picture = if can_render {
            PictureState::Queued
        } else {
            PictureState::Skipped
        };
        arrival
    }

    /// Queues the picture of download `id` and starts the worker when it
    /// is not running.
    pub(super) fn queue_picture(&self, id: String) {
        self.inner
            .pictures
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .queue
            .push_back(id);
        if self.inner.rendering.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        std::thread::spawn(move || me.work());
    }

    fn next_picture(&self) -> Option<String> {
        self.inner
            .pictures
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .queue
            .pop_front()
    }

    /// Renders queued pictures one at a time until none is left.
    fn work(&self) {
        loop {
            while let Some(id) = self.next_picture() {
                let done = self
                    .picture_of(&id)
                    .and_then(|png| self.rank_by_picture(&id, &png));
                if let Err(err) = done {
                    tracing::warn!(error = %err, "a download was not matched by picture");
                    self.picture_failed(&id);
                }
            }
            self.inner.rendering.store(false, Ordering::SeqCst);
            // A file queued while the worker was stopping starts it again.
            let more = !self
                .inner
                .pictures
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .queue
                .is_empty();
            if !more || self.inner.rendering.swap(true, Ordering::SeqCst) {
                return;
            }
        }
    }

    /// Renders the front picture of download `id`, or finds it cached.
    fn picture_of(&self, id: &str) -> CoreResult<PathBuf> {
        let renderer = self
            .renderer()
            .ok_or_else(|| CoreError::Unavailable("No picture renderer.".to_string()))?;
        let (root, config) = self
            .inner
            .studio
            .workspace_paths()
            .ok_or_else(|| CoreError::Unavailable("Open a workspace first.".to_string()))?;
        let (_, hash) = self.find(id)?;
        let key = if hash.len() >= 32 { &hash[..32] } else { id };
        let dir = root.join(&config.layout.cache).join(PICTURES_DIR);
        let cached = dir.join(format!("{key}.png"));
        if cached.is_file() {
            return Ok(cached);
        }
        let model = self.preview_file(id)?;
        renderer.render(&model, &dir, key)
    }

    /// Ranks every waiting send against the front picture `png` of download
    /// `id` and suggests the clear best, if any. Nothing is attached.
    ///
    /// # Errors
    ///
    /// Fails for an unknown download.
    pub fn rank_by_picture(&self, id: &str, png: &Path) -> CoreResult<Arrival> {
        let (arrival, _) = self.find(id)?;
        let seen = picture::object_in(png);
        let waiting = waiting(&self.inner.studio);
        let dirs = |p: &str| self.project_dir(p);
        let ranked = rank(
            &arrival.name,
            arrival.inner.as_deref(),
            seen.as_ref(),
            &waiting,
            &dirs,
        );
        let mut next = Arrival {
            suggestions: ranked,
            render: Some(png.display().to_string()),
            picture: if seen.is_some() {
                PictureState::Ready
            } else {
                PictureState::Failed
            },
            ..arrival
        };
        if let Some(best) = looks_like(&next.suggestions).cloned()
            && let Some(job) = waiting.iter().find(|j| j.id == best.job)
            && is_open(next.state)
        {
            next.state = ArrivalState::Review;
            next.reason = format!("Looks like {} ({}%)", best.asset_name, percent(best.score));
            suggest(&mut next, job);
        }
        Ok(self.replace(next))
    }

    fn picture_failed(&self, id: &str) {
        if let Ok((arrival, _)) = self.find(id) {
            self.replace(Arrival {
                picture: PictureState::Failed,
                ..arrival
            });
        }
    }

    /// Puts `arrival` in place of the one with its id, while that one still
    /// waits, and tells the app.
    fn replace(&self, arrival: Arrival) -> Arrival {
        let changed = {
            let mut state = self.inner.state();
            match state.arrivals.iter_mut().find(|a| a.id == arrival.id) {
                Some(slot) if is_open(slot.state) => {
                    *slot = arrival.clone();
                    true
                }
                _ => false,
            }
        };
        if changed {
            (self.inner.on_arrival)(&arrival);
        }
        arrival
    }
}
