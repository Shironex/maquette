//! Game mechanics overlaps: two system cards that share a verb, a look or
//! an input read as one system to a player, so a Game mechanics canvas
//! flags them on the cards and in the Overlaps panel.
//!
//! A system card is a note with `Verb:`, `Look:` or `Input:` lines (the
//! preset's cards have all three). Each line is boiled down to its main
//! word, and two cards with the same main word on the same line overlap:
//!
//! - **Verb:** the first word that is not a stop word (`draw to cast` is
//!   `draw`, `slow time around you` is `slow`).
//! - **Look:** the last such word, the thing seen (`blue glyph rings` is
//!   `ring`).
//! - **Input:** each key or button, split at commas, slashes, plus signs,
//!   `and` and `or`, with the gesture left out (`tap Shift` and `hold Shift`
//!   are both `shift`; `Q and E` is `q` and `e`; `left mouse` and
//!   `right mouse` stay apart).
//!
//! Words are lower case, split at anything but letters and digits, and a
//! plural reads as its singular (`rings`, `ring`; `bodies`, `body`). A line
//! still holding the preset's starter words is left out. The rules are
//! simple on purpose, so a person can predict them; `overlap/cases.json`
//! holds the cases both this and the mock backend's copy are tested on.
//!
//! The search is bounded: a card counts its first [`MAX_CARD_WORDS`] main
//! words, and it stops at [`MAX_OVERLAPS`] pairs in node id order, so a
//! full canvas of look-alike cards stays quick.
//!
//! A person's "Mark as fine" stores the pair with a key made from both
//! cards' titles and texts ([`FineMark`]); the flag stays away until either
//! card's words change, and only the person marks (no agent tool does).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{PlanFile, PlanNode, PlanPreset, presets};
use crate::error::{CoreError, CoreResult};

/// The most overlaps reported, so a canvas of look-alike cards stays readable.
pub const MAX_OVERLAPS: usize = 500;
/// The most main words of one card that count, in part and word order, so
/// one card of many keys can not make the check slow.
pub const MAX_CARD_WORDS: usize = 32;
/// The most pairs a canvas keeps marked as fine.
pub const MAX_FINE: usize = 4_000;

/// Which line of two system cards says the same.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum OverlapPart {
    Verb,
    Look,
    Input,
}

/// Two system cards that share a main word on one line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Overlap {
    /// The two node ids, `a` before `b`.
    pub a: String,
    pub b: String,
    pub part: OverlapPart,
    /// The main word they share.
    pub word: String,
    /// The person marked this pair as fine for the words they hold now.
    pub fine: bool,
}

/// A pair of cards the person marked as fine, for their words as they were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FineMark {
    /// The two node ids, `a` before `b`.
    pub a: String,
    pub b: String,
    /// [`pair_key`] of the two cards when marked.
    pub key: String,
}

const STOP_WORDS: [&str; 34] = [
    "a", "an", "the", "to", "of", "and", "or", "for", "with", "on", "in", "at", "by", "from",
    "into", "onto", "your", "you", "it", "its", "their", "them", "is", "are", "be", "as", "up",
    "out", "over", "around", "then", "than", "each", "every",
];

/// How a key is pressed, which says nothing about which key it is.
const GESTURES: [&str; 12] = [
    "tap", "hold", "press", "click", "double", "long", "quick", "release", "mash", "push", "held",
    "pressed",
];

/// `word` as its singular: `rings` to `ring`, `bodies` to `body`, `boxes`
/// to `box`; `glass`, `bus` and `axis` stay.
pub fn singular(word: &str) -> String {
    let n = word.chars().count();
    if n > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..word.len() - 3]);
    }
    for end in ["sses", "xes", "zes", "ches", "shes"] {
        if n > end.len() && word.ends_with(end) {
            return word[..word.len() - 2].to_string();
        }
    }
    if n > 3 && word.ends_with('s') && !["ss", "us", "is"].iter().any(|e| word.ends_with(e)) {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// The words of `value` in groups split at commas, slashes, plus signs,
/// ampersands, semicolons and the words `and` and `or`: lower case, plain
/// letters and digits.
fn groups(value: &str) -> Vec<Vec<String>> {
    let mut out = vec![Vec::new()];
    let mut word = String::new();
    let end_word = |word: &mut String, out: &mut Vec<Vec<String>>| {
        if word.is_empty() {
            return;
        }
        let w = std::mem::take(word);
        if w == "and" || w == "or" {
            out.push(Vec::new());
        } else if let Some(last) = out.last_mut() {
            last.push(w);
        }
    };
    for c in value.chars() {
        if c.is_alphanumeric() {
            word.extend(c.to_lowercase());
        } else {
            end_word(&mut word, &mut out);
            if matches!(c, ',' | '/' | '+' | '&' | ';') {
                out.push(Vec::new());
            }
        }
    }
    end_word(&mut word, &mut out);
    out
}

/// The main words of one line's `value`: one for a verb or a look, one per
/// key for an input. Empty when only stop words are left.
pub fn main_words(part: OverlapPart, value: &str) -> Vec<String> {
    let content = |words: &[String], drop_gestures: bool| -> Vec<String> {
        words
            .iter()
            .filter(|w| !STOP_WORDS.contains(&w.as_str()))
            .filter(|w| !drop_gestures || !GESTURES.contains(&w.as_str()))
            .map(|w| singular(w))
            .collect()
    };
    match part {
        OverlapPart::Verb | OverlapPart::Look => {
            let all: Vec<String> = groups(value).concat();
            let words = content(&all, false);
            let main = if part == OverlapPart::Verb {
                words.first()
            } else {
                words.last()
            };
            main.cloned().into_iter().collect()
        }
        OverlapPart::Input => {
            let mut keys: Vec<String> = groups(value)
                .iter()
                .map(|g| content(g, true).join(" "))
                .filter(|k| !k.is_empty())
                .collect();
            keys.sort();
            keys.dedup();
            keys
        }
    }
}

/// The part a line's label names, `Verb`, `Look` or `Input` in any case.
fn part_of(label: &str) -> Option<OverlapPart> {
    match label.trim().to_ascii_lowercase().as_str() {
        "verb" => Some(OverlapPart::Verb),
        "look" => Some(OverlapPart::Look),
        "input" => Some(OverlapPart::Input),
        _ => None,
    }
}

/// A system card's main words per part; `None` for a node that is not one.
fn card_words(node: &PlanNode, starters: &BTreeSet<String>) -> Option<Vec<(OverlapPart, String)>> {
    if node.kind != super::NodeKind::Note {
        return None;
    }
    let mut found = false;
    let mut words = Vec::new();
    for line in node.text.lines() {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        let Some(part) = part_of(label) else {
            continue;
        };
        found = true;
        if starters.contains(&value.trim().to_lowercase()) {
            continue;
        }
        words.extend(main_words(part, value).into_iter().map(|w| (part, w)));
    }
    found.then_some(words)
}

/// A short key for the words of two cards (in id order): FNV-1a over their
/// titles and texts as UTF-8, eight hex digits. The mock backend makes the
/// same key.
pub fn pair_key(a: &PlanNode, b: &PlanNode) -> String {
    let (a, b) = if a.id <= b.id { (a, b) } else { (b, a) };
    fnv(&format!(
        "{}\n{}\u{0}{}\n{}",
        a.title, a.text, b.title, b.text
    ))
}

/// FNV-1a over `text` as UTF-8, as eight hex digits.
pub(super) fn fnv(text: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in text.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{hash:08x}")
}

/// A card's main word with the line it is on.
type Word = (OverlapPart, String);

/// The overlaps of a Game mechanics canvas, in node id order, each marked
/// fine when the person marked its pair for the words it holds now. Other
/// presets have none, and skip the check. A canvas whose cards' words and
/// marks did not change since a recent check gets that answer again
/// ([`cache`]).
pub fn overlaps(plan: &PlanFile) -> Vec<Overlap> {
    if plan.preset != PlanPreset::Mechanics {
        return Vec::new();
    }
    cache::answer(plan, find)
}

/// What one check did, counted so a test can hold the check to its bounds
/// on any machine, however fast.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
struct Work {
    /// Cards walked as the first card of a pair.
    cards: usize,
    /// The most main words one card kept.
    most_words: usize,
    /// Pairs of cards looked at through a word they share.
    pairs: usize,
}

/// The check itself. Each card is paired only with the cards after it, a
/// card at a time, so the work stops once [`MAX_OVERLAPS`] are found.
fn find(plan: &PlanFile) -> Vec<Overlap> {
    find_counted(plan, &mut Work::default())
}

/// [`find`], adding what it did to `work`.
fn find_counted(plan: &PlanFile, work: &mut Work) -> Vec<Overlap> {
    let starters: BTreeSet<String> = presets::starter_values().map(str::to_lowercase).collect();
    let mut cards: BTreeMap<&str, (&PlanNode, Vec<Word>)> = BTreeMap::new();
    for node in &plan.nodes {
        let mut words = card_words(node, &starters).unwrap_or_default();
        words.sort();
        words.dedup();
        words.truncate(MAX_CARD_WORDS);
        work.most_words = work.most_words.max(words.len());
        if !words.is_empty() {
            cards.insert(&node.id, (node, words));
        }
    }
    // The cards holding each word, in id order (the cards are walked so).
    let mut by_word: BTreeMap<&Word, Vec<&str>> = BTreeMap::new();
    for (id, (_, words)) in &cards {
        for word in words {
            by_word.entry(word).or_default().push(id);
        }
    }
    let mut out = Vec::new();
    for (a, (na, words)) in &cards {
        work.cards += 1;
        // The cards after `a` sharing a word, each part once with its first word.
        let mut pairs: BTreeMap<(&str, OverlapPart), &str> = BTreeMap::new();
        for word in words {
            let ids = &by_word[word];
            let after = ids.partition_point(|id| id <= a);
            for b in &ids[after..] {
                work.pairs += 1;
                pairs.entry((b, word.0)).or_insert(&word.1);
            }
        }
        for ((b, part), word) in pairs {
            let key = pair_key(na, cards[b].0);
            let fine = plan
                .fine
                .iter()
                .any(|m| m.a == *a && m.b == b && m.key == key);
            out.push(Overlap {
                a: (*a).to_string(),
                b: b.to_string(),
                part,
                word: word.to_string(),
                fine,
            });
            if out.len() == MAX_OVERLAPS {
                return out;
            }
        }
    }
    out
}

/// The marks whose two nodes are both still on the canvas.
pub(super) fn kept_marks(plan: &PlanFile) -> Vec<FineMark> {
    let ids: BTreeSet<&str> = plan.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut marks: Vec<FineMark> = plan
        .fine
        .iter()
        .filter(|m| ids.contains(m.a.as_str()) && ids.contains(m.b.as_str()))
        .cloned()
        .collect();
    marks.sort_by(|x, y| (&x.a, &x.b).cmp(&(&y.a, &y.b)));
    marks
}

/// Checks the fine marks' shape: ids in order, one mark per pair, a hex key.
pub(super) fn check_fine(plan: &PlanFile) -> CoreResult<()> {
    if plan.fine.len() > MAX_FINE {
        return Err(CoreError::Invalid(format!(
            "A canvas keeps at most {MAX_FINE} overlaps marked as fine."
        )));
    }
    let mut pairs = BTreeSet::new();
    for mark in &plan.fine {
        let hex = !mark.key.is_empty()
            && mark.key.len() <= 16
            && mark.key.chars().all(|c| c.is_ascii_hexdigit());
        if mark.a >= mark.b || !hex || !pairs.insert((&mark.a, &mark.b)) {
            return Err(CoreError::Invalid(format!(
                "The overlap marked as fine for {} and {} is not a plain, unique pair.",
                mark.a, mark.b
            )));
        }
    }
    Ok(())
}

mod cache;

#[cfg(test)]
mod tests;
