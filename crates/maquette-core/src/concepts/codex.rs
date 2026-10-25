//! The Codex side of a concept image: the prompt it reads on standard
//! input and the `codex exec` arguments that attach the references.

use std::path::Path;

use super::{NewConcepts, resolve_ref};

/// The prompt Codex gets on standard input. It starts with "Generate an
/// image." and ends with where to save the result, because Codex keeps its
/// generated images in `~/.codex/generated_images/` and only copies one out
/// when asked.
pub fn prompt(new: &NewConcepts, index_in_batch: u32, out: &Path) -> String {
    let mut text = String::from("Generate an image.\n\n");
    text.push_str(new.prompt.trim());
    text.push_str("\n\n");
    text.push_str(&format!(
        "This is a concept for the direction \"{}\". Use a {} {} composition.",
        new.direction,
        new.aspect.ratio(),
        new.aspect.shape()
    ));
    if new.count > 1 {
        text.push_str(&format!(
            " It is image {index_in_batch} of {}, so make it a distinct take on the brief.",
            new.count
        ));
    }
    if !new.refs.is_empty() {
        text.push_str(
            " The attached images are references: follow their style, shapes and palette.",
        );
    }
    text.push_str("\n\n");
    text.push_str(&save_instructions(out));
    text
}

/// How every concept prompt ends: Codex keeps its images in its own
/// folder and only copies one out when asked, to this exact path.
pub(crate) fn save_instructions(out: &Path) -> String {
    format!(
        "Generate exactly one image and do nothing else in this folder. When it is ready, copy it out of ~/.codex/generated_images/ and save it as a PNG at exactly this path, creating the folder if needed:\n{}\n",
        out.display()
    )
}

/// The `codex exec` arguments. The prompt goes on standard input: `-i` takes
/// several files, so it would swallow a prompt given as an argument, and
/// `--` ends the options before the `-` that says "read standard input".
pub fn codex_args(project_dir: &Path, refs: &[String]) -> Vec<String> {
    let mut args: Vec<String> = ["exec", "--skip-git-repo-check", "-s", "workspace-write"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    for r in refs {
        args.push("-i".to_string());
        args.push(resolve_ref(project_dir, r).display().to_string());
    }
    args.push("--".to_string());
    args.push("-".to_string());
    args
}
