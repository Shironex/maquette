//! The compress step's material tweaks: a model with tweaks is compressed
//! from a tweaked copy in the cache's temp folder, written right before the
//! step's first call, so the raw export never changes. A WebP or AVIF model
//! with two texture sizes is compressed from a further copy that marks the
//! images of the smaller group (see `presets::image_passes`).

use std::path::{Path, PathBuf};

use super::execution::Execution;
use super::step::CompressModel;
use crate::materials::{apply_file, map_groups, mark_small_maps, unmark_small_maps};
use crate::pipeline::history::LogLevel;
use crate::pipeline::presets::{color_is_smaller, needs_split, resolve};
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

impl Execution {
    fn split_file(tmp: &Path, stem: &str) -> PathBuf {
        tmp.join(format!("{stem}.split.glb"))
    }

    /// True when `model` ships WebP or AVIF with color and data maps at
    /// different sizes, so it is compressed from a copy that marks the
    /// smaller group's images.
    fn splits(&self, model: &CompressModel) -> Option<bool> {
        if self.setup.textures.is_ktx2() {
            return None;
        }
        let preset = resolve(&self.setup.def.presets, &model.stem, self.setup.defaults);
        needs_split(&preset, map_groups(&model.input)).then(|| color_is_smaller(&preset))
    }

    /// The file the texture passes read for `model`: its marked copy, its
    /// tweaked copy, or the raw export.
    pub(super) fn compress_input(&self, model: &CompressModel, tmp: &Path) -> PathBuf {
        if self.splits(model).is_some() {
            Self::split_file(tmp, &model.stem)
        } else {
            self.tweaked_input(model, tmp)
        }
    }

    /// Writes the marked copy of every model that needs one, after the
    /// tweaked copies, so it carries the tweaks.
    pub(super) fn write_split(&mut self, spec: &CompressSpec, root: &Path) -> Result<(), String> {
        let tmp = self.tweak_dir();
        for model in self.compress_models(spec, root) {
            let Some(color_smaller) = self.splits(&model) else {
                continue;
            };
            let from = self.tweaked_input(&model, &tmp);
            mark_small_maps(&from, &Self::split_file(&tmp, &model.stem), color_smaller)?;
        }
        Ok(())
    }

    /// Takes the marks off the images of every compressed model that had
    /// them, so the shipped file keeps its image names.
    pub(super) fn unmark_outputs(
        &mut self,
        spec: &CompressSpec,
        root: &Path,
    ) -> Result<(), String> {
        for model in self.compress_models(spec, root) {
            if self.splits(&model).is_none() {
                continue;
            }
            let output = root.join(&spec.to).join(format!("{}.glb", model.shipped));
            unmark_small_maps(&output)?;
        }
        Ok(())
    }
}
