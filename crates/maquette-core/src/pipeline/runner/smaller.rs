//! The compress step's last check: a model its passes made larger than the
//! file they read ships as that file instead, so Compress never ships a
//! model bigger than its export. Tiny models are where it happens: a small
//! PNG or JPEG can come out of KTX2 larger than it went in.

use std::fs;
use std::path::Path;

use super::execution::Execution;
use crate::fs::write_atomic;
use crate::pipeline::history::LogLevel;
use crate::pipeline::{CompressSpec, StepDef};

fn kb(bytes: u64) -> String {
    format!("{:.0} KB", bytes as f64 / 1024.0)
}

impl Execution {
    /// Puts the compress input back over every output that came out
    /// larger, and logs each one with both sizes.
    pub(super) fn keep_smaller_outputs(
        &mut self,
        step: &StepDef,
        spec: &CompressSpec,
        root: &Path,
    ) -> Result<(), String> {
        let tmp = self.setup.cache_dir.join("tmp");
        for model in self.compress_models(spec, root) {
            // The tweaked copy when there is one: it is what the passes
            // read, with the person's material tweaks and no size marks.
            let input = self.tweaked_input(&model, &tmp);
            let output = root.join(&spec.to).join(format!("{}.glb", model.shipped));
            let (Ok(read), Ok(wrote)) = (fs::metadata(&input), fs::metadata(&output)) else {
                continue;
            };
            if wrote.len() <= read.len() {
                continue;
            }
            let bytes =
                fs::read(&input).map_err(|e| format!("Could not read {}. {e}", input.display()))?;
            write_atomic(&output, &bytes)
                .map_err(|e| format!("Could not write {}. {e}", output.display()))?;
            self.system(
                Some(&step.id),
                LogLevel::Warn,
                format!(
                    "{}.glb came out of Compress larger than its export ({} against {}), so it ships as exported, without compression.",
                    model.shipped,
                    kb(wrote.len()),
                    kb(read.len())
                ),
            );
        }
        Ok(())
    }
}
