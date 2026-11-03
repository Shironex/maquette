//! A change in a few words for the change log and the Changes panel:
//! "Added note: Frost veil", "Added 3 notes, moved 2 nodes".

use serde::{Deserialize, Serialize};

use super::ops::PlanOp;
use super::{NodeKind, PlanNode};

/// What kind of change a log line is, for its mark in the Changes panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeMark {
    Add,
    Edit,
    Move,
    Remove,
    /// Only links between nodes were drawn, changed or removed.
    Link,
    /// Several kinds at once.
    Mixed,
    Undo,
    Rename,
    Create,
    /// A person kept (accepted) agent changes.
    Keep,
    /// A person applied the camera plan to `layout.json`.
    Apply,
    /// The camera plan was brought up to a changed `layout.json`.
    Rebase,
}

fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Note => "note",
        NodeKind::Image => "picture",
        NodeKind::Link => "link",
        NodeKind::Asset => "asset",
        NodeKind::Direction => "Lab direction",
        NodeKind::Stop => "stop",
    }
}

/// A node's one line name: its title, else its first line of text, else
/// what it links.
pub fn node_name(node: &PlanNode) -> String {
    let title = node.title.trim();
    let first = node.text.trim().lines().next().unwrap_or_default().trim();
    let stop = node.stop.as_ref().map_or("", |s| s.name.trim());
    let name = [stop, title, first]
        .into_iter()
        .find(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| node.asset.clone())
        .or_else(|| node.direction.clone())
        .unwrap_or_else(|| format!("empty {}", kind_name(node.kind)));
    clip_words(&name, NAME_MAX)
}

/// The longest name a summary shows, in characters.
const NAME_MAX: usize = 60;

/// `text` cut to `max` characters at the end of a word, with an ellipsis;
/// a single word longer than that is cut where it reaches the limit.
fn clip_words(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max - 1).collect();
    let cut = match head.rfind(char::is_whitespace) {
        Some(at) if head[..at].chars().count() >= max / 3 => &head[..at],
        _ => head.as_str(),
    };
    format!(
        "{}\u{2026}",
        cut.trim_end_matches(|c: char| c.is_whitespace() || c == ',' || c == '.')
    )
}

#[derive(Default)]
struct Counts<'a> {
    added: Vec<&'a PlanNode>,
    removed: Vec<&'a PlanNode>,
    moved: Vec<&'a PlanNode>,
    edited: Vec<&'a PlanNode>,
    links: usize,
}

fn moved_only(before: &PlanNode, after: &PlanNode) -> bool {
    let placed = PlanNode {
        x: before.x,
        y: before.y,
        ..after.clone()
    };
    placed == *before
}

fn count(ops: &[PlanOp]) -> Counts<'_> {
    let mut c = Counts::default();
    for op in ops {
        match op {
            PlanOp::AddNode { node } => c.added.push(node),
            PlanOp::RemoveNode { node } => c.removed.push(node),
            PlanOp::SetNode { before, after } if moved_only(before, after) => c.moved.push(after),
            PlanOp::SetNode { after, .. } => c.edited.push(after),
            PlanOp::AddEdge { .. } | PlanOp::RemoveEdge { .. } | PlanOp::SetEdge { .. } => {
                c.links += 1;
            }
            // The base moves only with a rebase or an Apply, which say so
            // themselves, and the settings say what they did.
            PlanOp::SetCamera { .. }
            | PlanOp::SetAutoLayout { .. }
            | PlanOp::MarkFine { .. }
            | PlanOp::UnmarkFine { .. } => {}
        }
    }
    c
}

/// "2 notes", "1 picture", "3 nodes" for mixed kinds.
fn nodes_phrase(nodes: &[&PlanNode]) -> String {
    let n = nodes.len();
    let same = nodes.windows(2).all(|w| w[0].kind == w[1].kind);
    let noun = match nodes.first() {
        Some(first) if same => kind_name(first.kind),
        _ => "node",
    };
    if n == 1 {
        format!("1 {noun}")
    } else if noun == "Lab direction" {
        format!("{n} Lab directions")
    } else {
        format!("{n} {noun}s")
    }
}

/// The mark of a change made of `ops`.
pub fn mark(ops: &[PlanOp]) -> ChangeMark {
    let c = count(ops);
    let kinds = [
        (!c.added.is_empty(), ChangeMark::Add),
        (!c.removed.is_empty(), ChangeMark::Remove),
        (!c.moved.is_empty(), ChangeMark::Move),
        (!c.edited.is_empty(), ChangeMark::Edit),
    ];
    let present: Vec<ChangeMark> = kinds
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, m)| *m)
        .collect();
    match present.as_slice() {
        [] if c.links > 0 => ChangeMark::Link,
        [] => ChangeMark::Edit,
        [one] => *one,
        _ => ChangeMark::Mixed,
    }
}

/// A change in a few words. One node added, edited, moved or removed is
/// named; more are counted.
pub fn summary(ops: &[PlanOp]) -> String {
    let c = count(ops);
    let single = |verb: &str, node: &PlanNode| {
        format!("{verb} {}: {}", kind_name(node.kind), node_name(node))
    };
    let groups = [&c.added, &c.removed, &c.moved, &c.edited];
    let busy = groups.iter().filter(|g| !g.is_empty()).count();
    if busy == 1 {
        for (verb, group) in [
            ("Added", &c.added),
            ("Removed", &c.removed),
            ("Moved", &c.moved),
            ("Edited", &c.edited),
        ] {
            if let [one] = group.as_slice() {
                return single(verb, one);
            }
        }
    }
    let mut parts = Vec::new();
    for (verb, group) in [
        ("added", &c.added),
        ("removed", &c.removed),
        ("moved", &c.moved),
        ("edited", &c.edited),
    ] {
        if !group.is_empty() {
            parts.push(format!("{verb} {}", nodes_phrase(group)));
        }
    }
    if c.links > 0 {
        let noun = if c.links == 1 { "link" } else { "links" };
        parts.push(format!("changed {} {noun} between nodes", c.links));
    }
    let line = parts.join(", ");
    let mut chars = line.chars();
    chars.next().map_or_else(
        || "No change".to_string(),
        |first| first.to_uppercase().chain(chars).collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::clip_words;

    #[test]
    fn long_names_end_at_a_word_with_an_ellipsis() {
        let text = "Keep it under the porch roof so the lanterns read before the moon does";
        let clipped = clip_words(text, 60);
        assert_eq!(
            clipped,
            "Keep it under the porch roof so the lanterns read before\u{2026}"
        );
        assert!(clipped.chars().count() <= 60);
        assert_eq!(clip_words("Short", 60), "Short");
        let word = "a".repeat(80);
        assert_eq!(clip_words(&word, 10).chars().count(), 10);
        assert_eq!(
            clip_words("Ünïcödé wörds hérè ãnd thêrè", 12),
            "Ünïcödé\u{2026}"
        );
    }
}
