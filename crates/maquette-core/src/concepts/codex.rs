//! The Codex side of a concept image: the prompt it reads on standard
//! input and the `codex exec` arguments that attach the references.

use std::path::Path;

use super::style::StyleBoard;
use super::{NewConcepts, resolve_ref};

/// The prompt Codex gets on standard input for a concept. With a style
/// board, its images come after the direction's own references.
pub fn prompt(
    new: &NewConcepts,
    index_in_batch: u32,
    out: &Path,
    style: Option<&StyleBoard>,
) -> String {
    let mut lead = String::from(new.prompt.trim());
    lead.push_str("\n\n");
    lead.push_str(&format!(
        "This is a concept for the direction \"{}\". Use a {} {} composition.",
        new.direction,
        new.aspect.ratio(),
        new.aspect.shape()
    ));
    if new.count > 1 {
        lead.push_str(&format!(
            " It is image {index_in_batch} of {}, so make it a distinct take on the brief.",
            new.count
        ));
    }
    let styled = style.is_some_and(|s| !s.images.is_empty());
    match new.refs.len() {
        0 => {}
        1 if styled => lead.push_str(
            " The first attached image is a reference: follow its style, shapes and palette.",
        ),
        n if styled => lead.push_str(&format!(
            " The first {n} attached images are references: follow their style, shapes and palette."
        )),
        _ => lead.push_str(
            " The attached images are references: follow their style, shapes and palette.",
        ),
    }
    assemble(&lead, style, out)
}

/// Puts every Codex image prompt together, the one place that does: it
/// starts with "Generate an image.", then the brief's own text, then the
/// project's style as a separate note that says the brief leads, and ends
/// with where to save the result, because Codex keeps its generated images
/// in `~/.codex/generated_images/` and only copies one out when asked.
///
/// The style board's images are attached after the brief's own (see
/// [`attached`]), so the note names them as the last ones.
pub fn assemble(lead: &str, style: Option<&StyleBoard>, out: &Path) -> String {
    let mut text = String::from("Generate an image.\n\n");
    text.push_str(lead.trim());
    text.push_str("\n\n");
    if let Some(note) = style.and_then(style_note) {
        text.push_str(&note);
        text.push_str("\n\n");
    }
    text.push_str(&save_instructions(out));
    text
}

/// The project style note, or `None` for an empty board.
fn style_note(board: &StyleBoard) -> Option<String> {
    if board.is_empty() {
        return None;
    }
    let mut lines = vec![
        "Project style: this project keeps one look across all its images. Apply it on top of the brief above; where the two disagree, the brief wins.".to_string(),
    ];
    let line = board.line.trim();
    if !line.is_empty() {
        lines.push(format!("Style: {}.", line.trim_end_matches('.')));
    }
    if !board.palette.is_empty() {
        lines.push(format!("Palette: {}.", board.palette.join(", ")));
    }
    match board.images.len() {
        0 => {}
        1 => lines.push(
            "The last attached image is the project's style board: take only its look from it (materials, light, colour, line and finish), never its subject, objects or layout.".to_string(),
        ),
        n => lines.push(format!(
            "The last {n} attached images are the project's style board: take only their look from them (materials, light, colour, line and finish), never their subjects, objects or layout."
        )),
    }
    Some(lines.join("\n"))
}

/// How every concept prompt ends: Codex keeps its images in its own
/// folder and only copies one out when asked, to this exact path.
pub(crate) fn save_instructions(out: &Path) -> String {
    format!(
        "Generate exactly one image and do nothing else in this folder. When it is ready, copy it out of ~/.codex/generated_images/ and save it as a PNG at exactly this path, creating the folder if needed:\n{}\n",
        out.display()
    )
}

/// The images a job attaches: the brief's own references first, then the
/// style board's, so [`assemble`] can name the board's as the last ones.
pub fn attached(refs: &[String], style: Option<&StyleBoard>) -> Vec<String> {
    let mut out = refs.to_vec();
    if let Some(board) = style {
        out.extend(board.files());
    }
    out
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
