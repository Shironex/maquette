//! Answering a download the watcher would not place by itself: attaching
//! it to an asset (into the inbox for it, as a named export goes) or
//! leaving it for good, and the GLB copy its thumbnail renders from.

use std::path::{Path, PathBuf};

use super::{Arrival, ArrivalState, Arrivals, PREVIEW_DIR, is_fbx, is_open};
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::handback::read_model;

impl Arrivals {
    /// Attaches a download that waits for a look or for its asset to
    /// `asset` of `project`: it goes into the inbox for it, as a named
    /// export does (for the asset's waiting By hand send, else aimed at the
    /// asset), and accepting it there closes the asset's sends still
    /// waiting. Agents call it too, by name: nothing reaches an asset
    /// without someone asking for it.
    pub fn attach(&self, id: &str, project: &str, asset: &str) -> CoreResult<Arrival> {
        {
            let mut state = self.inner.state();
            if state.attaching.iter().any(|a| a == id) {
                return Err(CoreError::Invalid(
                    "That download is being attached already.".to_string(),
                ));
            }
            state.attaching.push(id.to_string());
        }
        let done = self.attach_to(id, project, asset);
        self.inner.state().attaching.retain(|a| a != id);
        done
    }

    fn attach_to(&self, id: &str, project: &str, asset: &str) -> CoreResult<Arrival> {
        let (arrival, hash) = self.find(id)?;
        if !is_open(arrival.state) {
            return Err(CoreError::Invalid(format!(
                "{} was already taken care of.",
                arrival.name
            )));
        }
        if is_fbx(Path::new(&arrival.path)) {
            return Err(CoreError::Invalid(format!(
                "{} is an FBX. Export it as GLB: the inbox checks read GLB.",
                arrival.name
            )));
        }
        let landed = self
            .inner
            .studio
            .attach(project, asset, Path::new(&arrival.path))?;
        self.mark_seen(&hash, landed.job.as_deref());
        self.drop_preview(id);
        let done = Arrival {
            state: ArrivalState::Attached,
            job: landed.job,
            project: Some(project.to_string()),
            reason: format!("You attached it to {}", landed.asset_name),
            asset: Some(landed.asset),
            asset_name: Some(landed.asset_name),
            inbox_file: Some(landed.inbox_file),
            ..arrival
        };
        Ok(self.remember(done, hash))
    }

    /// Leaves a download alone for good: "Not this one" for a download
    /// waiting for a look, "Not a Maquette model" for one asking which
    /// asset it is. It is remembered, so it is never asked about again.
    pub fn dismiss(&self, id: &str) -> CoreResult<Arrival> {
        let (arrival, hash) = self.find(id)?;
        if !is_open(arrival.state) && arrival.state != ArrivalState::NeedsGlb {
            return Err(CoreError::Invalid(format!(
                "{} was already taken care of.",
                arrival.name
            )));
        }
        self.mark_seen(&hash, None);
        self.drop_preview(id);
        let reason = match (&arrival.state, &arrival.asset_name) {
            (ArrivalState::Review, Some(asset)) => format!("You said it is not {asset}"),
            _ => "Not a model for Maquette".to_string(),
        };
        let done = Arrival {
            state: ArrivalState::Dismissed,
            reason,
            ..arrival
        };
        let mut state = self.inner.state();
        if let Some(slot) = state.arrivals.iter_mut().find(|a| a.id == id) {
            *slot = done.clone();
        }
        Ok(done)
    }

    /// A GLB copy of a download that waits, in the app's data folder, for
    /// its thumbnail to be rendered from: the download may be a zip, and
    /// it is only ever read. Copies of downloads no longer waiting go.
    pub fn preview_file(&self, id: &str) -> CoreResult<PathBuf> {
        let (arrival, _) = self.find(id)?;
        if !is_open(arrival.state) {
            return Err(CoreError::Invalid(format!(
                "{} was already taken care of.",
                arrival.name
            )));
        }
        let dir = self.inner.seen_dir.join(PREVIEW_DIR);
        self.prune_previews(&dir);
        let file = dir.join(format!("{id}.glb"));
        if file.is_file() {
            return Ok(file);
        }
        let model = read_model(Path::new(&arrival.path))?;
        std::fs::create_dir_all(&dir).map_err(|e| CoreError::io(&dir, e))?;
        write_atomic(&file, &model.bytes).map_err(|e| CoreError::io(&file, e))?;
        Ok(file)
    }

    /// Removes the preview copies of downloads no longer waiting.
    fn prune_previews(&self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let keep: Vec<String> = self
            .inner
            .state()
            .arrivals
            .iter()
            .filter(|a| is_open(a.state))
            .map(|a| format!("{}.glb", a.id))
            .collect();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !keep.contains(&name) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    fn drop_preview(&self, id: &str) {
        let file = self
            .inner
            .seen_dir
            .join(PREVIEW_DIR)
            .join(format!("{id}.glb"));
        let _ = std::fs::remove_file(file);
    }
}
