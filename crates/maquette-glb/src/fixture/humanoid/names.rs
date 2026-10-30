//! The joint names each fixture naming gives the Mixamo skeleton.

use super::Naming;

/// The name of a Mixamo joint (without its prefix) under `naming`.
pub(super) fn rename(bare: &str, naming: Naming) -> String {
    let (side, rest) = bare
        .strip_prefix("Left")
        .map(|r| (Some(('l', "left", "L")), r))
        .or_else(|| {
            bare.strip_prefix("Right")
                .map(|r| (Some(('r', "right", "R")), r))
        })
        .unwrap_or((None, bare));
    match naming {
        Naming::Mixamo => format!("mixamorig:{bare}"),
        Naming::Vrm => vrm_name(side.map(|s| s.1), rest).unwrap_or_else(|| bare.to_string()),
        Naming::Unreal => {
            unreal_name(side.map(|s| s.0), rest).unwrap_or_else(|| format!("{bare}_end"))
        }
        Naming::Tripo => {
            tripo_name(side.map(|s| s.2), rest).unwrap_or_else(|| format!("{bare}_end"))
        }
        Naming::Descriptive => {
            descriptive_name(side.map(|s| (s.2, s.1)), rest).unwrap_or_else(|| bare.to_string())
        }
    }
}

/// `side` is the letter and the word: ("L", "left").
fn descriptive_name(side: Option<(&str, &str)>, rest: &str) -> Option<String> {
    let body = match rest {
        "Hips" => return Some("Hips".into()),
        "Spine" => return Some("Spine".into()),
        "Spine1" => return Some("Chest".into()),
        "Spine2" => return Some("Upper chest".into()),
        "Neck" => return Some("Neck".into()),
        "Head" => return Some("Head".into()),
        "HeadTop_End" => return Some("Head top end".into()),
        "Shoulder" => "shoulder",
        "Arm" => "arm",
        "ForeArm" => "elbow",
        "Hand" => "wrist",
        "UpLeg" => "leg",
        "Leg" => "knee",
        "Foot" => "ankle",
        "ToeBase" => "toe",
        "Toe_End" => "toe end",
        _ => {
            let (f, n) = finger(rest)?;
            let f = if f == "Pinky" { "Little" } else { f };
            let segment = ["Proximal", "Intermediate", "Distal", "End"].get(n.checked_sub(1)?)?;
            return Some(format!("{f}{segment}_{}", side?.0));
        }
    };
    let (_, word) = side?;
    let word = if word == "left" { "Left" } else { "Right" };
    Some(format!("{word} {body}"))
}

fn finger(rest: &str) -> Option<(&str, usize)> {
    let tail = rest.strip_prefix("Hand")?;
    let digit = tail.chars().last()?.to_digit(10)?;
    Some((&tail[..tail.len() - 1], usize::try_from(digit).ok()?))
}

fn vrm_name(side: Option<&str>, rest: &str) -> Option<String> {
    let body = match rest {
        "Hips" => "hips",
        "Spine" => "spine",
        "Spine1" => "chest",
        "Spine2" => "upperChest",
        "Neck" => "neck",
        "Head" => "head",
        "Shoulder" => "Shoulder",
        "Arm" => "UpperArm",
        "ForeArm" => "LowerArm",
        "Hand" => "Hand",
        "UpLeg" => "UpperLeg",
        "Leg" => "LowerLeg",
        "Foot" => "Foot",
        "ToeBase" => "Toes",
        _ => {
            let (f, n) = finger(rest)?;
            let segments = if f == "Thumb" {
                ["Metacarpal", "Proximal", "Distal"]
            } else {
                ["Proximal", "Intermediate", "Distal"]
            };
            let segment = segments.get(n.checked_sub(1)?)?;
            let f = if f == "Pinky" { "Little" } else { f };
            return Some(format!("{}{f}{segment}", side?));
        }
    };
    Some(side.map_or_else(|| body.to_string(), |s| format!("{s}{body}")))
}

fn unreal_name(side: Option<char>, rest: &str) -> Option<String> {
    let body = match rest {
        "Hips" => return Some("pelvis".into()),
        "Spine" => return Some("spine_01".into()),
        "Spine1" => return Some("spine_02".into()),
        "Spine2" => return Some("spine_03".into()),
        "Neck" => return Some("neck_01".into()),
        "Head" => return Some("head".into()),
        "Shoulder" => "clavicle",
        "Arm" => "upperarm",
        "ForeArm" => "lowerarm",
        "Hand" => "hand",
        "UpLeg" => "thigh",
        "Leg" => "calf",
        "Foot" => "foot",
        "ToeBase" => "ball",
        _ => {
            let (f, n) = finger(rest)?;
            return (n <= 3).then(|| format!("{}_0{n}_{}", f.to_lowercase(), side.unwrap_or('c')));
        }
    };
    Some(format!("{body}_{}", side?))
}

fn tripo_name(side: Option<&str>, rest: &str) -> Option<String> {
    let body = match rest {
        "Hips" => return Some("Hip".into()),
        "Spine" => return Some("Waist".into()),
        "Spine1" => return Some("Spine01".into()),
        "Spine2" => return Some("Spine02".into()),
        "Neck" => return Some("NeckTwist01".into()),
        "Head" => return Some("Head".into()),
        "Shoulder" => "Clavicle",
        "Arm" => "Upperarm",
        "ForeArm" => "Forearm",
        "Hand" => "Hand",
        "UpLeg" => "Thigh",
        "Leg" => "Calf",
        "Foot" => "Foot",
        "ToeBase" => "ToeBase",
        _ => return None,
    };
    Some(format!("{}_{body}", side?))
}
