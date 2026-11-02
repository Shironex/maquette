//! Where an agent's nodes go: next to what they relate to, clear of every
//! other node, never stacked at 0, 0. Node sizes are guessed the way the
//! view guesses them before a node has drawn (`apps/web/src/state/plans.ts`).

use super::{NodeKind, PlanNode};

/// The gap kept between nodes.
const GAP: i32 = 32;
/// The step a search moves down or across.
const STEP: i32 = 20;

/// A node's box on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Rect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

impl Rect {
    fn clear_of(&self, other: &Self) -> bool {
        self.x + self.w + GAP <= other.x
            || other.x + other.w + GAP <= self.x
            || self.y + self.h + GAP <= other.y
            || other.y + other.h + GAP <= self.y
    }
}

/// How wide a node draws when the file keeps no width.
pub(super) fn default_width(kind: NodeKind) -> i32 {
    match kind {
        NodeKind::Note | NodeKind::Direction => 220,
        NodeKind::Image => 240,
        NodeKind::Link => 260,
        NodeKind::Asset => 248,
    }
}

/// How tall a node is likely to draw: a note grows with its text.
fn guess_height(node: &PlanNode, width: i32) -> i32 {
    match node.kind {
        NodeKind::Note => {
            let per_line = usize::try_from((width - 24) / 7).unwrap_or(30).max(10);
            let lines: usize = node
                .text
                .lines()
                .map(|l| l.chars().count().div_ceil(per_line).max(1))
                .sum();
            let title = if node.title.trim().is_empty() { 0 } else { 20 };
            let tall = 48 + title + 18 * i32::try_from(lines).unwrap_or(40).min(40);
            tall.max(110)
        }
        NodeKind::Image => 250,
        NodeKind::Link => 130,
        NodeKind::Asset => 120,
        NodeKind::Direction => 190,
    }
}

pub(super) fn rect_of(node: &PlanNode) -> Rect {
    let w = node
        .width
        .and_then(|w| i32::try_from(w).ok())
        .unwrap_or_else(|| default_width(node.kind));
    Rect {
        x: node.x,
        y: node.y,
        w,
        h: guess_height(node, w),
    }
}

/// A free top left corner for a `w` by `h` node: to the right of `anchor`
/// (a node it relates to) and down from its top, else to the right of
/// everything, else at the origin. Columns further right are tried when a
/// column is full.
pub(super) fn free_spot(anchor: Option<Rect>, taken: &[Rect], w: i32, h: i32) -> (i32, i32) {
    let (start_x, top) = match anchor {
        Some(a) => (a.x + a.w + GAP * 2, a.y),
        None => match (
            taken.iter().map(|r| r.x + r.w).max(),
            taken.iter().map(|r| r.y).min(),
        ) {
            (Some(right), Some(top)) => (right + GAP * 2, top),
            _ => return (0, 0),
        },
    };
    let free = |x: i32, y: i32| {
        let me = Rect { x, y, w, h };
        taken.iter().all(|t| me.clear_of(t))
    };
    for column in 0..12 {
        let x = start_x + column * (w + GAP * 2);
        for row in 0..120 {
            let y = top + row * STEP;
            if free(x, y) {
                return (x, y);
            }
        }
    }
    (start_x, top)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32) -> Rect {
        Rect {
            x,
            y,
            w: 220,
            h: 110,
        }
    }

    #[test]
    fn a_spot_lands_right_of_its_anchor_and_clear_of_others() {
        let anchor = rect(0, 0);
        let (x, y) = free_spot(Some(anchor), &[anchor], 220, 110);
        assert_eq!((x, y), (284, 0));
        let blocker = rect(284, 0);
        let (x, y) = free_spot(Some(anchor), &[anchor, blocker], 220, 110);
        assert_eq!(x, 284);
        assert!(y >= 110 + GAP, "below the blocker: {y}");
        assert_eq!(free_spot(None, &[], 220, 110), (0, 0));
        let (x, _) = free_spot(None, &[anchor, rect(500, 40)], 220, 110);
        assert_eq!(x, 720 + GAP * 2, "right of everything");
    }

    #[test]
    fn a_long_note_is_taller() {
        let mut node = PlanNode {
            id: "n-1".into(),
            kind: NodeKind::Note,
            x: 0,
            y: 0,
            width: None,
            title: String::new(),
            text: "short".into(),
            image: None,
            url: None,
            asset: None,
            direction: None,
            by: None,
        };
        assert_eq!(rect_of(&node).h, 110);
        node.text = "word ".repeat(200);
        assert!(rect_of(&node).h > 300);
    }
}
