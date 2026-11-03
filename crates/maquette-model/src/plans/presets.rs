//! What a preset puts on a new canvas. Presets only set the start: a few
//! plain notes and links a person replaces, laid out in the shape the
//! preset is for. They are not node kinds of their own, and every canvas
//! holds every kind of node. Brainstorm starts blank.
//!
//! The view's mock backend keeps a copy of these words
//! (`apps/web/src/ipc/mock/plan-presets.ts`). `presets/seeds.json` holds
//! these seeds as data, kept equal to them by a test here, and the mock's
//! test compares its seeds with it card for card.

use super::log::new_id;
use super::{EdgeKind, NodeKind, PlanEdge, PlanNode, PlanPreset, Side};
use crate::settings::game::ProjectTarget;

/// One starter note: where it sits, its title and its text.
struct Card {
    x: i32,
    y: i32,
    title: &'static str,
    text: &'static str,
}

/// A starter link between two cards, by their place in the list.
struct Join {
    from: usize,
    to: usize,
    from_side: Side,
    to_side: Side,
    kind: EdgeKind,
}

/// A preset's starter cards and links.
struct Seed {
    cards: &'static [Card],
    joins: &'static [Join],
}

/// Columns and rows of a seed, wide enough for the 220 pixel notes and the
/// gap the canvas keeps between nodes.
const COL: i32 = 280;
const ROW: i32 = 240;

const fn card(col: i32, row: i32, title: &'static str, text: &'static str) -> Card {
    Card {
        x: col * COL,
        y: row * ROW,
        title,
        text,
    }
}

const fn lead(from: usize, to: usize) -> Join {
    Join {
        from,
        to,
        from_side: Side::Right,
        to_side: Side::Left,
        kind: EdgeKind::LeadsTo,
    }
}

/// A character above a beat, joined from its bottom to the beat's top.
const fn down(from: usize, to: usize) -> Join {
    Join {
        from,
        to,
        from_side: Side::Bottom,
        to_side: Side::Top,
        kind: EdgeKind::Relates,
    }
}

/// A place below a beat, joined from its top to the beat's bottom.
const fn up(from: usize, to: usize) -> Join {
    Join {
        from,
        to,
        from_side: Side::Top,
        to_side: Side::Bottom,
        kind: EdgeKind::Relates,
    }
}

/// Story: beats left to right in the middle row, characters above and
/// places below, each joined to the beat it first shows in.
const STORY_JOINS: [Join; 7] = [
    lead(2, 3),
    lead(3, 4),
    lead(4, 5),
    down(0, 2),
    down(1, 3),
    up(6, 2),
    up(7, 5),
];

const STORY_GAME: [Card; 8] = [
    card(
        0,
        0,
        "Character: Main character",
        "Who the player is and what they want. Link their asset here once it is on the Board.",
    ),
    card(
        1,
        0,
        "Character: Rival",
        "Who or what stands in the way, and why.",
    ),
    card(
        0,
        1,
        "Beat 1: Opening",
        "Where it starts and what the player sees first.",
    ),
    card(
        1,
        1,
        "Beat 2: Trouble",
        "What goes wrong and pulls the player in.",
    ),
    card(
        2,
        1,
        "Beat 3: Turn",
        "The surprise that changes the plan. Draw two links out of a beat to branch.",
    ),
    card(
        3,
        1,
        "Beat 4: Ending",
        "How it ends and what the player keeps.",
    ),
    card(
        0,
        2,
        "Place: First place",
        "Where the opening happens. Drop a picture or link a Lab direction.",
    ),
    card(3, 2, "Place: Last place", "Where it ends."),
];

const STORY_WEBSITE: [Card; 8] = [
    card(
        0,
        0,
        "Character: Main subject",
        "What the site is about: a person, a product or a place.",
    ),
    card(
        1,
        0,
        "Character: Visitor",
        "Who scrolls through it and what they came for.",
    ),
    card(
        0,
        1,
        "Beat 1: First view",
        "What a visitor sees before they scroll.",
    ),
    card(
        1,
        1,
        "Beat 2: Hook",
        "The detail that makes them keep scrolling.",
    ),
    card(
        2,
        1,
        "Beat 3: Reveal",
        "The view the scroll builds up to. Draw two links out of a beat to branch.",
    ),
    card(
        3,
        1,
        "Beat 4: Close",
        "The last view and what you want them to do next.",
    ),
    card(
        0,
        2,
        "Place: Opening scene",
        "Where the first camera stop looks. Drop a picture or link a Lab direction.",
    ),
    card(3, 2, "Place: Final scene", "Where the tour ends."),
];

/// Game mechanics: one card per system with its verb, look and input, and
/// a note on keeping them apart.
const MECHANICS: [Card; 4] = [
    card(
        0,
        0,
        "System: Core action",
        "Verb: what the player does most\nLook: how it reads on screen\nInput: the key or button",
    ),
    card(
        1,
        0,
        "System: Movement",
        "Verb: how the player gets around\nLook: how it reads on screen\nInput: the key or button",
    ),
    card(
        2,
        0,
        "System: Defence",
        "Verb: how the player stays alive\nLook: how it reads on screen\nInput: the key or button",
    ),
    card(
        0,
        1,
        "Keep them apart",
        "One verb, one look and one input per system. Two systems that share any of them read as one. Add a card for each new system.",
    ),
];

fn seed_of(preset: PlanPreset, target: ProjectTarget) -> Seed {
    match (preset, target) {
        (PlanPreset::Brainstorm, _) => Seed {
            cards: &[],
            joins: &[],
        },
        (PlanPreset::Story, ProjectTarget::Game) => Seed {
            cards: &STORY_GAME,
            joins: &STORY_JOINS,
        },
        (PlanPreset::Story, ProjectTarget::Website) => Seed {
            cards: &STORY_WEBSITE,
            joins: &STORY_JOINS,
        },
        (PlanPreset::Mechanics, _) => Seed {
            cards: &MECHANICS,
            joins: &[],
        },
    }
}

/// The words a Game mechanics starter card holds on its lines until a
/// person writes their own (`what the player does most`), which the overlap
/// check leaves out.
pub fn starter_values() -> impl Iterator<Item = &'static str> {
    MECHANICS
        .iter()
        .flat_map(|c| c.text.lines())
        .filter_map(|line| line.split_once(": ").map(|(_, value)| value))
}

/// The preset's name as the app shows it.
pub fn label(preset: PlanPreset) -> &'static str {
    match preset {
        PlanPreset::Brainstorm => "Brainstorm",
        PlanPreset::Story => "Story",
        PlanPreset::Mechanics => "Game mechanics",
    }
}

/// The starter nodes and links of `preset` for a project of `target`, with
/// fresh ids, made by `by` (an agent's name, or `None` for a person).
pub fn seed(
    preset: PlanPreset,
    target: ProjectTarget,
    by: Option<&str>,
) -> (Vec<PlanNode>, Vec<PlanEdge>) {
    let seed = seed_of(preset, target);
    let by = by.map(str::to_string);
    let nodes: Vec<PlanNode> = seed
        .cards
        .iter()
        .map(|c| PlanNode {
            id: new_id("n"),
            kind: NodeKind::Note,
            x: c.x,
            y: c.y,
            width: None,
            title: c.title.to_string(),
            text: c.text.to_string(),
            image: None,
            url: None,
            asset: None,
            direction: None,
            by: by.clone(),
            person_wrote: false,
            stop: None,
        })
        .collect();
    let edges = seed
        .joins
        .iter()
        .map(|j| PlanEdge {
            id: new_id("e"),
            from: nodes[j.from].id.clone(),
            to: nodes[j.to].id.clone(),
            from_side: Some(j.from_side),
            to_side: Some(j.to_side),
            kind: j.kind,
            label: None,
            by: by.clone(),
        })
        .collect();
    (nodes, edges)
}

#[cfg(test)]
mod tests;
