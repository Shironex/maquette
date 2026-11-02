//! The exact difference between two lists of camera stops: which stops are
//! added, removed, moved in the order or changed, and the `layout.json`
//! lines that change. A moved stop shows twice in the lines, taken out where
//! it was and put in where it goes, so a reordering is never hidden.
//!
//! A stop counts as moved when it leaves the longest run of stops that keep
//! their order: putting the moon second moves the moon, not every stop it
//! jumped over. Stops that only shift because of it keep their place mark.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::layout::LayoutStop;

/// One setting of a stop before and after, as a person reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FieldChange {
    /// name, position, target, fov or models.
    pub field: String,
    pub before: String,
    pub after: String,
}

/// What happens to one stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StopChange {
    pub id: String,
    pub name: String,
    /// The plan's stop node, for stops that stay or come.
    pub node: Option<String>,
    /// Its place in the tour before and after, from 1; none when it is new
    /// or goes.
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub added: bool,
    pub removed: bool,
    /// It leaves the longest run of stops that keep their order.
    pub moved: bool,
    pub fields: Vec<FieldChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum DiffKind {
    Same,
    Add,
    Remove,
}

/// One line of `layout.json`'s stops in the diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
    /// Why the line is there: "moves to 2", "moved from 4", "new", "removed", "changed".
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StopDiff {
    /// The stops in their new order, then the ones that go.
    pub stops: Vec<StopChange>,
    pub lines: Vec<DiffLine>,
    pub added: u32,
    pub removed: u32,
    pub moved: u32,
    /// Stops whose settings change, moved or not.
    pub changed: u32,
}

impl StopDiff {
    /// True when nothing changes.
    pub fn is_empty(&self) -> bool {
        self.added + self.removed + self.moved + self.changed == 0
    }

    /// "1 stop added · 1 moved · 0 removed · 2 changed".
    pub fn counts_line(&self) -> String {
        let noun = if self.added == 1 { "stop" } else { "stops" };
        format!(
            "{} {noun} added · {} moved · {} removed · {} changed",
            self.added, self.moved, self.removed, self.changed
        )
    }
}

/// A number as `layout.json` reads it: at most three decimals, no trailing zeros.
fn num(v: f32) -> String {
    let text = format!("{v:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

fn vec3(v: [f32; 3]) -> String {
    format!("[{}, {}, {}]", num(v[0]), num(v[1]), num(v[2]))
}

fn models(m: Option<&Vec<String>>) -> String {
    m.map_or_else(|| "what the view holds".into(), |list| list.join(", "))
}

/// The settings that differ between two states of one stop.
fn field_changes(before: &LayoutStop, after: &LayoutStop) -> Vec<FieldChange> {
    let mut out = Vec::new();
    let mut push = |field: &str, b: String, a: String| {
        if b != a {
            out.push(FieldChange {
                field: field.into(),
                before: b,
                after: a,
            });
        }
    };
    push("name", before.name.clone(), after.name.clone());
    push("position", vec3(before.position), vec3(after.position));
    push("target", vec3(before.target), vec3(after.target));
    push("fov", num(before.fov), num(after.fov));
    push(
        "models",
        models(before.models.as_ref()),
        models(after.models.as_ref()),
    );
    out
}

/// One stop as a line of `layout.json`.
fn line_text(stop: &LayoutStop) -> String {
    let models = stop.models.as_ref().map_or_else(
        || "null".to_string(),
        |m| {
            let names: Vec<String> = m.iter().map(|n| format!("{n:?}")).collect();
            format!("[{}]", names.join(", "))
        },
    );
    format!(
        "{{ \"id\": {:?}, \"name\": {:?}, \"position\": {}, \"target\": {}, \"fov\": {}, \"models\": {models} }}",
        stop.id,
        stop.name,
        vec3(stop.position),
        vec3(stop.target),
        num(stop.fov)
    )
}

/// The ids of the longest run of stops in both lists in the same order.
fn kept_order<'a>(before: &[&'a str], after: &[&str]) -> BTreeSet<&'a str> {
    let (n, m) = (before.len(), after.len());
    let mut best = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            best[i][j] = if before[i] == after[j] {
                best[i + 1][j + 1] + 1
            } else {
                best[i + 1][j].max(best[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, BTreeSet::new());
    while i < n && j < m {
        if before[i] == after[j] {
            out.insert(before[i]);
            i += 1;
            j += 1;
        } else if best[i + 1][j] >= best[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

fn place(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(u32::MAX)
}

fn line(kind: DiffKind, stop: &LayoutStop, note: Option<String>) -> DiffLine {
    DiffLine {
        kind,
        text: line_text(stop),
        note,
    }
}

/// How `before` becomes `after`. `nodes` are the plan's stop node ids of
/// `after`, in the same order (or empty).
pub fn diff_stops(before: &[LayoutStop], after: &[LayoutStop], nodes: &[String]) -> StopDiff {
    let b_ids: Vec<&str> = before.iter().map(|s| s.id.as_str()).collect();
    let a_ids: Vec<&str> = after.iter().map(|s| s.id.as_str()).collect();
    let kept = kept_order(&b_ids, &a_ids);
    let at_before = |id: &str| b_ids.iter().position(|b| *b == id);
    let at_after = |id: &str| a_ids.iter().position(|a| *a == id);
    let mut out = StopDiff::default();
    for (i, stop) in after.iter().enumerate() {
        let old = at_before(&stop.id);
        let fields = old
            .map(|o| field_changes(&before[o], stop))
            .unwrap_or_default();
        let moved = old.is_some() && !kept.contains(stop.id.as_str());
        out.added += u32::from(old.is_none());
        out.moved += u32::from(moved);
        out.changed += u32::from(!fields.is_empty());
        out.stops.push(StopChange {
            id: stop.id.clone(),
            name: stop.name.clone(),
            node: nodes.get(i).cloned(),
            from: old.map(place),
            to: Some(place(i)),
            added: old.is_none(),
            removed: false,
            moved,
            fields,
        });
    }
    for (i, stop) in before.iter().enumerate() {
        if at_after(&stop.id).is_none() {
            out.removed += 1;
            out.stops.push(StopChange {
                id: stop.id.clone(),
                name: stop.name.clone(),
                node: None,
                from: Some(place(i)),
                to: None,
                added: false,
                removed: true,
                moved: false,
                fields: Vec::new(),
            });
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < before.len() || j < after.len() {
        if let Some(b) = before.get(i).filter(|b| !kept.contains(b.id.as_str())) {
            let note = at_after(&b.id)
                .map_or_else(|| "removed".into(), |to| format!("moves to {}", place(to)));
            out.lines.push(line(DiffKind::Remove, b, Some(note)));
            i += 1;
        } else if let Some(a) = after.get(j).filter(|a| !kept.contains(a.id.as_str())) {
            let note = at_before(&a.id).map_or_else(
                || "new".into(),
                |from| format!("moved from {}", place(from)),
            );
            out.lines.push(line(DiffKind::Add, a, Some(note)));
            j += 1;
        } else if let (Some(b), Some(a)) = (before.get(i), after.get(j)) {
            if b == a {
                out.lines.push(line(DiffKind::Same, a, None));
            } else {
                out.lines
                    .push(line(DiffKind::Remove, b, Some("changed".into())));
                out.lines
                    .push(line(DiffKind::Add, a, Some("changed".into())));
            }
            i += 1;
            j += 1;
        } else {
            break;
        }
    }
    out
}
