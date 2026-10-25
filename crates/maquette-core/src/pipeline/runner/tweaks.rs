//! The compress step's material tweaks: a model with tweaks is compressed
//! from a tweaked copy in the cache's temp folder, written right before the
//! step's first call, so the raw export never changes.

use std::path::{Path, PathBuf};

use super::execution::Execution;
use super::step::CompressModel;
use crate::materials::apply_file;
use crate::pipeline::history::LogLevel;
use crate::pipeline::{CompressSpec, StepDef};

impl Execution {
    fn tweak_dir(&self) -> PathBuf {
        self.setup.cache_dir.join("tmp")
    }

    fn tweak_file(tmp: &Path, stem: &str) -> PathBuf {
        tmp.join(format!("{stem}.look.glb"))
    }

    /// The file Compress reads for `model`: its tweaked copy when it has
    /// material tweaks, else the raw export itself.
    pub(super) fn tweaked_input(&self, model: &CompressModel, tmp: &Path) -> PathBuf {
        if self
            .setup
            .materials
            .contains_key(&model.stem.to_lowercase())
        {
            Self::tweak_file(tmp, &model.stem)
        } else {
            model.input.clone()
        }
    }

    /// Writes the tweaked copy of every model with tweaks and logs what each
    /// changed.
    pub(super) fn write_tweaked(
        &mut self,
        step: &StepDef,
        spec: &CompressSpec,
        root: &Path,
    ) -> Result<(), String> {
        if self.setup.materials.is_empty() {
            return Ok(());
        }
        let tmp = self.tweak_dir();
        for model in self.compress_models(spec, root) {
            let Some(look) = self.setup.materials.get(&model.stem.to_lowercase()) else {
                continue;
            };
            let applied = apply_file(&model.input, &Self::tweak_file(&tmp, &model.stem), look)
                .map_err(|err| err.to_string())?;
            let (level, line) = match applied.materials {
                0 => (
                    LogLevel::Warn,
                    format!(
                        "{} has material tweaks, but none of its materials has their names.",
                        model.stem
                    ),
                ),
                n => {
                    let maps = match applied.maps_dropped {
                        0 => String::new(),
                        1 => ", 1 metal and roughness map replaced".to_string(),
                        m => format!(", {m} metal and roughness maps replaced"),
                    };
                    let what = if n == 1 { "material" } else { "materials" };
                    (
                        LogLevel::Info,
                        format!("{} material tweaks on {n} {what}{maps}", model.stem),
                    )
                }
            };
            self.system(Some(&step.id), level, line);
        }
        Ok(())
    }
}
