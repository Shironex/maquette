//! Export GLB from the Inspector: run the step that writes the raw folder
//! on its own, into a scratch folder, and keep only the one model asked
//! for. The rest of the raw folder stays as the last full run left it.

use std::fs;
use std::path::{Path, PathBuf};

use super::{PipelineDef, StepKind};
use crate::error::{CoreError, CoreResult};
use crate::fs::write_atomic;
use crate::inspect::folders;

/// The id the one step run gets, so its cache record never mixes with the
/// pipeline's own export step.
pub const STEP_ID: &str = "export-one";

fn normal(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

/// The pipeline cut down to its export step, writing into `out_dir`.
///
/// The export step is the Blender step with an `--out` argument whose
/// outputs land in the raw folder (the compress step's input).
pub fn def(
    def: &PipelineDef,
    project_dir: &Path,
    model: &str,
    out_dir: &Path,
) -> CoreResult<PipelineDef> {
    let root = def.root_dir(project_dir);
    let (raw_dir, _) = folders(project_dir, def);
    let raw = normal(raw_dir.strip_prefix(&root).unwrap_or(&raw_dir));
    let writes_raw = |pattern: &str| {
        Path::new(pattern)
            .parent()
            .is_some_and(|parent| normal(parent) == raw)
    };
    let step = def
        .steps
        .iter()
        .filter(|s| s.kind == StepKind::Blender && s.args.iter().any(|a| a == "--out"))
        .find(|s| s.outputs.iter().any(|o| writes_raw(o)))
        .ok_or_else(|| {
            CoreError::Invalid(format!(
                "No pipeline step exports into {raw} with an --out folder, so I cannot export one model on its own. Run the pipeline instead."
            ))
        })?;
    let mut step = step.clone();
    let mut args = step.args.iter();
    let mut replaced = Vec::with_capacity(step.args.len());
    while let Some(arg) = args.next() {
        replaced.push(arg.clone());
        if arg == "--out" {
            args.next();
            replaced.push(out_dir.display().to_string());
        }
    }
    step.args = replaced;
    step.id = STEP_ID.to_string();
    step.name = format!("{} for {}", step.name, crate::inspect::model_label(model));
    step.depends_on.clear();
    // The scratch folder is outside the project, so the step lists no
    // outputs; [`keep`] checks the one file that matters.
    step.outputs.clear();
    Ok(PipelineDef {
        steps: vec![step],
        ..def.clone()
    })
}

/// Copies `<out_dir>/<model>.glb` over the model's raw file and returns
/// where it went.
pub fn keep(
    def: &PipelineDef,
    project_dir: &Path,
    model: &str,
    out_dir: &Path,
) -> CoreResult<PathBuf> {
    let file = format!("{model}.glb");
    let made = out_dir.join(&file);
    if !made.is_file() {
        let mut wrote: Vec<String> = fs::read_dir(out_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| n.ends_with(".glb"))
                    .collect()
            })
            .unwrap_or_default();
        wrote.sort();
        return Err(CoreError::Invalid(if wrote.is_empty() {
            format!("The export ran but wrote no {file}.")
        } else {
            format!(
                "The export ran but wrote no {file}. It wrote {}.",
                wrote.join(", ")
            )
        }));
    }
    let bytes = fs::read(&made).map_err(|e| CoreError::io(&made, e))?;
    let (raw_dir, _) = folders(project_dir, def);
    let target = raw_dir.join(&file);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
    }
    write_atomic(&target, &bytes).map_err(|e| CoreError::io(&target, e))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectTemplate;
    use crate::pipeline::template::for_template;

    #[test]
    fn the_export_step_runs_alone_into_the_scratch_folder() {
        let full = for_template(ProjectTemplate::FloatingDiorama);
        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("scratch");
        let one = def(&full, dir.path(), "shop_house", &out).expect("def");
        assert_eq!(one.steps.len(), 1);
        let step = &one.steps[0];
        assert_eq!(step.id, STEP_ID);
        assert_eq!(step.name, "Export GLB for Shop house");
        assert!(step.depends_on.is_empty());
        let at = step.args.iter().position(|a| a == "--out").expect("--out");
        assert_eq!(step.args[at + 1], out.display().to_string());
        assert!(!step.args.iter().any(|a| a == "web/raw"));

        // Only the model asked for replaces its raw file.
        fs::create_dir_all(&out).expect("scratch");
        fs::write(out.join("shop_house.glb"), b"glTF new").expect("made");
        fs::write(out.join("tree.glb"), b"glTF other").expect("made");
        let raw = dir.path().join("web/raw");
        fs::create_dir_all(&raw).expect("raw");
        fs::write(raw.join("tree.glb"), b"glTF old").expect("old");
        let kept = keep(&full, dir.path(), "shop_house", &out).expect("keep");
        assert_eq!(fs::read(kept).expect("read"), b"glTF new");
        assert_eq!(fs::read(raw.join("tree.glb")).expect("read"), b"glTF old");
        let missing = keep(&full, dir.path(), "lantern", &out).expect_err("missing");
        assert!(
            missing.to_string().contains("shop_house.glb, tree.glb"),
            "{missing}"
        );
    }

    #[test]
    fn a_pipeline_without_an_export_step_says_so() {
        let mut full = for_template(ProjectTemplate::FloatingDiorama);
        full.steps.retain(|s| s.id != "export");
        let dir = tempfile::tempdir().expect("temp dir");
        let err = def(&full, dir.path(), "shop_house", dir.path()).expect_err("no step");
        assert!(err.to_string().contains("web/raw"), "{err}");
    }
}
