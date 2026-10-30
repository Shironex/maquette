use super::*;
use crate::inbox::vrm::{SpringKind, SpringPreset};

/// The ponytail's centre line on a real character, glTF metres.
const PONYTAIL: [[f64; 3]; 9] = [
    [-0.1576, 1.5071, -0.1965],
    [-0.1828, 1.3692, -0.231],
    [-0.2105, 1.2365, -0.2471],
    [-0.2569, 1.1309, -0.2733],
    [-0.2899, 1.0244, -0.2969],
    [-0.3384, 0.9209, -0.2999],
    [-0.3687, 0.8059, -0.2966],
    [-0.3441, 0.6733, -0.2919],
    [-0.3515, 0.5331, -0.288],
];

/// A strand on the other side of the head.
const STRAND: [[f64; 3]; 3] = [
    [0.1609, 1.576, -0.194],
    [0.19, 1.52, -0.23],
    [0.2296, 1.4566, -0.2743],
];

fn candidate(id: &str, kind: SpringKind, root: &str, line: &[[f64; 3]]) -> SpringCandidate {
    SpringCandidate {
        id: id.to_string(),
        kind,
        root_bone: root.to_string(),
        line: line.to_vec(),
        length: 1.0,
        drop: 1.0,
        vertices: 100,
        confidence: 0.9,
        joints: 6,
        color: "#e8553f".to_string(),
        small: false,
    }
}

fn found(candidates: Vec<SpringCandidate>) -> FoundSprings {
    FoundSprings {
        candidates,
        sheet: "candidates.png".to_string(),
        input: "normalized.glb".to_string(),
        height: 1.75,
    }
}

fn pick(id: &str, kind: SpringKind, root: &str, line: &[[f64; 3]]) -> SpringSpec {
    SpringSpec {
        id: id.to_string(),
        kind,
        root_bone: root.to_string(),
        line: line.to_vec(),
        joints: Some(7),
        preset: Some(SpringPreset::Soft),
        stiffness: None,
        gravity_power: None,
        drag_force: Some(0.3),
        hit_radius: None,
    }
}

fn shifted(line: &[[f64; 3]], by: [f64; 3]) -> Vec<[f64; 3]> {
    line.iter()
        .map(|p| [p[0] + by[0], p[1] + by[1], p[2] + by[2]])
        .collect()
}

#[test]
fn a_pick_whose_part_is_there_as_it_was_stays_as_it_is() {
    let now = found(vec![
        candidate("hair-1", SpringKind::Hair, "head", &PONYTAIL),
        candidate("hair-2", SpringKind::Hair, "head", &STRAND),
    ]);
    let picks = [pick("hair-1", SpringKind::Hair, "head", &PONYTAIL)];
    assert_eq!(
        hold(&picks, &now),
        Held::Matched {
            springs: picks.to_vec(),
            carried: Vec::new()
        }
    );
}

#[test]
fn a_pick_whose_part_changed_root_and_id_is_carried_over() {
    // The ponytail was cloth from the upper chest; the finder now reads it
    // as hair from the head, its line a little off and sampled anew.
    let now = found(vec![
        candidate(
            "hair-1",
            SpringKind::Hair,
            "head",
            &shifted(&PONYTAIL, [0.01, 0.0, -0.01]),
        ),
        candidate("hair-2", SpringKind::Hair, "head", &STRAND),
    ]);
    let old = pick("cloth-1", SpringKind::Cloth, "upperChest", &PONYTAIL);
    let Held::Matched { springs, carried } = hold(std::slice::from_ref(&old), &now) else {
        panic!("carried over");
    };
    assert_eq!(carried.len(), 1);
    assert_eq!(
        (carried[0].from.as_str(), carried[0].to.as_str()),
        ("cloth-1", "hair-1")
    );
    assert_eq!(carried[0].root_bone, "head");
    assert!(carried[0].overlap >= 0.99, "{carried:?}");
    let spring = &springs[0];
    assert_eq!(spring.id, "hair-1");
    assert_eq!(spring.root_bone, "head");
    assert_eq!(spring.line, now.candidates[0].line);
    // The person's choices stay.
    assert_eq!(spring.kind, SpringKind::Cloth);
    assert_eq!(spring.joints, Some(7));
    assert_eq!(spring.preset, Some(SpringPreset::Soft));
    assert_eq!(spring.drag_force, Some(0.3));
}

#[test]
fn a_pick_with_no_clear_part_now_is_stale() {
    // Gone: only the strand is left.
    let gone = found(vec![candidate("hair-1", SpringKind::Hair, "head", &STRAND)]);
    let old = pick("cloth-1", SpringKind::Cloth, "upperChest", &PONYTAIL);
    assert_eq!(
        hold(std::slice::from_ref(&old), &gone),
        Held::Stale(vec!["cloth-1".to_string()])
    );
    // Only half of it: the part split, so neither half is clearly it.
    let half = found(vec![
        candidate("hair-1", SpringKind::Hair, "head", &PONYTAIL[..5]),
        candidate("hair-2", SpringKind::Hair, "head", &PONYTAIL[4..]),
    ]);
    assert!(matches!(
        hold(std::slice::from_ref(&old), &half),
        Held::Stale(_)
    ));
    // Hanging from somewhere else, 20 cm away.
    let moved = found(vec![candidate(
        "hair-1",
        SpringKind::Hair,
        "head",
        &shifted(&PONYTAIL, [0.2, 0.0, 0.0]),
    )]);
    assert!(matches!(
        hold(std::slice::from_ref(&old), &moved),
        Held::Stale(_)
    ));
}

#[test]
fn two_picks_never_carry_onto_one_part() {
    let now = found(vec![candidate(
        "hair-1",
        SpringKind::Hair,
        "head",
        &PONYTAIL,
    )]);
    let picks = [
        pick("hair-1", SpringKind::Hair, "head", &PONYTAIL),
        pick("cloth-1", SpringKind::Cloth, "upperChest", &PONYTAIL),
    ];
    assert_eq!(hold(&picks, &now), Held::Stale(vec!["cloth-1".to_string()]));
}

#[test]
fn line_gap_reads_how_far_one_line_lies_from_another() {
    assert!(line_gap(&PONYTAIL, &PONYTAIL) < 1e-12);
    let off = shifted(&PONYTAIL, [0.0, 0.0, 0.1]);
    assert!((line_gap(&off, &PONYTAIL) - 0.1).abs() < 0.02);
    assert!(line_gap(&[], &PONYTAIL).is_infinite());
    assert!(line_gap(&PONYTAIL, &PONYTAIL[..1]).is_infinite());
}
