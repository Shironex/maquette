use super::super::bones::PairMatch;
use super::*;

fn bone(name: &str, parent: Option<&str>, head: [f32; 3]) -> RestBone {
    RestBone {
        name: name.into(),
        parent: parent.map(String::from),
        head,
        tail: [head[0], head[1] + 0.1, head[2]],
    }
}

fn pair(humanoid: &str, target: &str) -> BonePair {
    BonePair {
        bone: humanoid.into(),
        source: Some(humanoid.into()),
        target: Some(target.into()),
        how: PairMatch::ByName,
    }
}

fn rig() -> Vec<RestBone> {
    vec![
        bone("hips", None, [0.0, 1.0, 0.0]),
        bone("skirt_01", Some("hips"), [0.1, 0.9, 0.0]),
        bone("skirt_02", Some("skirt_01"), [0.1, 0.8, 0.0]),
        bone("hand.R", Some("hips"), [-0.6, 1.0, 0.0]),
        bone("root_helper", None, [0.0, 0.0, 0.0]),
    ]
}

fn map() -> Vec<BonePair> {
    vec![pair("Hips", "hips"), pair("RightHand", "hand.R")]
}

#[test]
fn a_stance_reads_from_how_far_the_arms_drop() {
    assert_eq!(shape(0.0), RestShape::TPose);
    assert_eq!(shape(47.0), RestShape::APose);
    assert_eq!(shape(88.0), RestShape::Other);
    let level = drop_of(Some([0.0, 1.0, 0.0]), Some([1.0, 1.0, 0.0])).expect("drop");
    assert!(level.abs() < 0.01);
    let down = drop_of(Some([0.0, 1.0, 0.0]), Some([1.0, 0.0, 0.0])).expect("drop");
    assert!((down - 45.0).abs() < 0.01);
    assert!((arms_drop([Some(40.0), Some(50.0)]) - 45.0).abs() < 0.01);
    assert!(arms_drop([None, None]).abs() < f32::EPSILON);
}

#[test]
fn bones_only_on_the_model_say_what_carries_them() {
    let saved = [FollowChoice {
        bone: "skirt_01".into(),
        follows: Some("hand.R".into()),
    }];
    let only = only_bones(&rig(), &map(), &saved);
    assert_eq!(only.len(), 3);
    assert_eq!(only[0].bone, "skirt_01");
    assert_eq!(only[0].carried_by.as_deref(), Some("hips"));
    assert_eq!(only[0].follows.as_deref(), Some("hand.R"));
    assert_eq!(
        only[1].carried_by.as_deref(),
        Some("hips"),
        "through skirt_01"
    );
    assert_eq!(only[2].carried_by, None, "nothing above it moves");
}

#[test]
fn a_bone_over_a_driven_one_cannot_follow_another() {
    let bones = vec![
        bone("root", None, [0.0, 0.0, 0.0]),
        bone("hips", Some("root"), [0.0, 1.0, 0.0]),
    ];
    let map = [pair("Hips", "hips")];
    let only = only_bones(&bones, &map, &[]);
    assert!(!only[0].free);
    let choice = [FollowChoice {
        bone: "root".into(),
        follows: Some("hips".into()),
    }];
    assert!(follow_pairs(&choice, &bones, &map).is_empty());
}

#[test]
fn only_real_changes_reach_the_bake() {
    let choose = |bone: &str, follows: &str| FollowChoice {
        bone: bone.into(),
        follows: Some(follows.into()),
    };
    let pairs = follow_pairs(
        &[
            choose("skirt_01", "hand.R"),
            choose("skirt_02", "hips"),
            choose("root_helper", "not_a_bone"),
            choose("hand.R", "hips"),
        ],
        &rig(),
        &map(),
    );
    assert_eq!(pairs, [("skirt_01".to_string(), "hand.R".to_string())]);
}

#[test]
fn this_retargets_choices_win_and_back_to_parent_clears_a_saved_one() {
    let saved = [FollowChoice {
        bone: "a".into(),
        follows: Some("x".into()),
    }];
    let picked = [FollowChoice {
        bone: "a".into(),
        follows: None,
    }];
    assert!(merged(&saved, &picked).is_empty());
    assert_eq!(merged(&saved, &[]), saved);
}
