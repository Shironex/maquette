//! The Mixamo skeleton the character models are rigged with.

/// The 65 joint Mixamo skeleton: name, parent, translation from the parent.
/// T pose, Y up, facing +Z, metres. The character's left is +X.
pub(super) fn skeleton() -> Vec<(String, Option<usize>, [f32; 3])> {
    let mut joints: Vec<(String, Option<usize>, [f32; 3])> = Vec::new();
    let add = |joints: &mut Vec<(String, Option<usize>, [f32; 3])>,
               name: &str,
               parent: Option<usize>,
               t: [f32; 3]| {
        joints.push((format!("mixamorig:{name}"), parent, t));
        joints.len() - 1
    };
    let hips = add(&mut joints, "Hips", None, [0.0, 1.0, 0.0]);
    let spine = add(&mut joints, "Spine", Some(hips), [0.0, 0.1, 0.0]);
    let spine1 = add(&mut joints, "Spine1", Some(spine), [0.0, 0.12, 0.0]);
    let spine2 = add(&mut joints, "Spine2", Some(spine1), [0.0, 0.12, 0.0]);
    let neck = add(&mut joints, "Neck", Some(spine2), [0.0, 0.14, 0.0]);
    let head = add(&mut joints, "Head", Some(neck), [0.0, 0.1, 0.0]);
    add(&mut joints, "HeadTop_End", Some(head), [0.0, 0.18, 0.0]);
    for (side, s) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        let shoulder = add(
            &mut joints,
            &format!("{side}Shoulder"),
            Some(spine2),
            [s * 0.06, 0.1, 0.0],
        );
        let arm = add(
            &mut joints,
            &format!("{side}Arm"),
            Some(shoulder),
            [s * 0.12, 0.0, 0.0],
        );
        let fore = add(
            &mut joints,
            &format!("{side}ForeArm"),
            Some(arm),
            [s * 0.26, 0.0, 0.0],
        );
        let hand = add(
            &mut joints,
            &format!("{side}Hand"),
            Some(fore),
            [s * 0.24, 0.0, 0.0],
        );
        for (finger, first) in [
            ("Thumb", [s * 0.03, 0.0, 0.03]),
            ("Index", [s * 0.08, 0.0, 0.025]),
            ("Middle", [s * 0.08, 0.0, 0.0]),
            ("Ring", [s * 0.075, 0.0, -0.02]),
            ("Pinky", [s * 0.07, 0.0, -0.04]),
        ] {
            let mut parent = add(
                &mut joints,
                &format!("{side}Hand{finger}1"),
                Some(hand),
                first,
            );
            for n in 2..=4 {
                parent = add(
                    &mut joints,
                    &format!("{side}Hand{finger}{n}"),
                    Some(parent),
                    [s * 0.03, 0.0, 0.0],
                );
            }
        }
    }
    for (side, s) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        let up = add(
            &mut joints,
            &format!("{side}UpLeg"),
            Some(hips),
            [s * 0.09, -0.05, 0.0],
        );
        let leg = add(
            &mut joints,
            &format!("{side}Leg"),
            Some(up),
            [0.0, -0.42, 0.0],
        );
        let foot = add(
            &mut joints,
            &format!("{side}Foot"),
            Some(leg),
            [0.0, -0.42, 0.0],
        );
        let toe = add(
            &mut joints,
            &format!("{side}ToeBase"),
            Some(foot),
            [0.0, -0.08, 0.1],
        );
        add(
            &mut joints,
            &format!("{side}Toe_End"),
            Some(toe),
            [0.0, 0.0, 0.06],
        );
    }
    joints
}

pub(super) fn world_positions(joints: &[(String, Option<usize>, [f32; 3])]) -> Vec<[f32; 3]> {
    let mut out: Vec<[f32; 3]> = Vec::with_capacity(joints.len());
    for (_, parent, t) in joints {
        let base = parent.map_or([0.0; 3], |p| out[p]);
        out.push([base[0] + t[0], base[1] + t[1], base[2] + t[2]]);
    }
    out
}
