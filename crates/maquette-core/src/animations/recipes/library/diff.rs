//! What a recipe changes against a library revision, in plain words, for
//! the save back sheet: the name, the base clip, and each change and blank
//! added, removed or edited (matched by id). Which prop fills a prop blank
//! is not counted, since asset ids belong to each project.

use crate::recipes::{BlankKind, LayerKind, RecipeBlank, RecipeFile, RecipeLayer};

fn kind_word(kind: LayerKind) -> &'static str {
    match kind {
        LayerKind::Reach => "reach",
        LayerKind::Nudge => "nudge",
        LayerKind::LookAt => "look at",
        LayerKind::Wave => "wave",
    }
}

fn layer_words(layer: &RecipeLayer) -> String {
    format!(
        "the {} on {} ({})",
        kind_word(layer.kind),
        layer.bone,
        layer.id
    )
}

fn blank_words(blank: &RecipeBlank) -> String {
    let kind = match blank.kind {
        BlankKind::Prop => "prop",
        BlankKind::Point => "point",
    };
    format!("the {kind} blank {} ({})", blank.id, blank.label)
}

/// The settings of a change that differ, by name.
fn layer_edits(a: &RecipeLayer, b: &RecipeLayer) -> Vec<&'static str> {
    let (x, y) = (&a.settings, &b.settings);
    let mut out = Vec::new();
    if a.kind != b.kind {
        out.push("kind");
    }
    if a.bone != b.bone {
        out.push("bone");
    }
    let pairs: [(&'static str, bool); 8] = [
        ("weight", x.weight != y.weight),
        ("start", x.start != y.start),
        ("end", x.end != y.end),
        ("target", x.target != y.target),
        ("rotation", x.rotation != y.rotation),
        ("offset", x.offset != y.offset),
        ("cycles", x.cycles != y.cycles),
        ("amount", x.amount != y.amount),
    ];
    out.extend(pairs.into_iter().filter(|(_, d)| *d).map(|(n, _)| n));
    out
}

/// The fields of a blank that differ, by name. Which prop fills a prop
/// blank is left out: asset ids belong to each project.
fn blank_edits(a: &RecipeBlank, b: &RecipeBlank) -> Vec<&'static str> {
    let pairs: [(&'static str, bool); 3] = [
        ("label", a.label != b.label),
        ("kind", a.kind != b.kind),
        ("point", a.point != b.point),
    ];
    pairs
        .into_iter()
        .filter(|(_, d)| *d)
        .map(|(n, _)| n)
        .collect()
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// What `after` changes against `before`, a line each; for no `before` (a
/// new library recipe), one line saying what it holds. Empty when they
/// hold the same name, base, blanks and changes.
pub fn changes(before: Option<&RecipeFile>, after: &RecipeFile) -> Vec<String> {
    let Some(before) = before else {
        return vec![format!(
            "A new library recipe on {} with {} and {}.",
            after.base.name,
            plural(after.layers.len(), "change", "changes"),
            plural(after.blanks.len(), "blank", "blanks")
        )];
    };
    let mut out = Vec::new();
    if before.name != after.name {
        out.push(format!(
            "Renames it from {} to {}.",
            before.name, after.name
        ));
    }
    let (b, a) = (&before.base, &after.base);
    if b.name != a.name || b.frames != a.frames || b.hash != a.hash {
        out.push(format!(
            "Changes the base clip from {} ({} frames) to {} ({} frames).",
            b.name, b.frames, a.name, a.frames
        ));
    }
    for layer in &after.layers {
        match before.layers.iter().find(|l| l.id == layer.id) {
            None => out.push(format!("Adds {}.", layer_words(layer))),
            Some(old) => {
                let edits = layer_edits(old, layer);
                if !edits.is_empty() {
                    out.push(format!(
                        "Edits {}: {}.",
                        layer_words(layer),
                        edits.join(", ")
                    ));
                }
            }
        }
    }
    for old in &before.layers {
        if !after.layers.iter().any(|l| l.id == old.id) {
            out.push(format!("Removes {}.", layer_words(old)));
        }
    }
    let same_ids = before.layers.len() == after.layers.len()
        && before
            .layers
            .iter()
            .zip(&after.layers)
            .all(|(x, y)| x.id == y.id);
    let same_set = before.layers.len() == after.layers.len()
        && before
            .layers
            .iter()
            .all(|x| after.layers.iter().any(|y| y.id == x.id));
    if same_set && !same_ids {
        out.push("Lays the changes over the clip in another order.".to_string());
    }
    for blank in &after.blanks {
        match before.blanks.iter().find(|x| x.id == blank.id) {
            None => out.push(format!("Adds {}.", blank_words(blank))),
            Some(old) => {
                let edits = blank_edits(old, blank);
                if !edits.is_empty() {
                    out.push(format!(
                        "Edits {}: {}.",
                        blank_words(blank),
                        edits.join(", ")
                    ));
                }
            }
        }
    }
    for old in &before.blanks {
        if !after.blanks.iter().any(|x| x.id == old.id) {
            out.push(format!("Removes {}.", blank_words(old)));
        }
    }
    out
}
