//! Descriptive joint names, the kind a person types: `Left leg`,
//! `Left elbow`, `Upper arm L`, `ThumbProximal_L`, `Hips`. A name is split
//! into words (at spaces, `_`, `.`, `-`, case changes and digits), its side
//! read from `left`, `right`, `l` or `r`, and its joint from the words:
//! elbow is the lower arm, knee the lower leg, wrist the hand and ankle
//! the foot. A plain `leg` or `arm` is the upper one.
//!
//! These are guesses. The bone map only takes one for a bone no exact
//! naming filled, when exactly one joint guesses that bone, and the
//! readiness says so.

use crate::analyse::bare_joint_name;

/// Words that mark a helper rather than a bone: twist and roll bones, IK
/// targets, chain ends.
const NOT_BONES: [&str; 14] = [
    "end", "top", "tip", "twist", "roll", "ik", "fk", "pole", "target", "ctrl", "control", "nub",
    "helper", "root",
];

const FINGERS: [(&str, &str); 7] = [
    ("thumb", "Thumb"),
    ("index", "Index"),
    ("middle", "Middle"),
    ("ring", "Ring"),
    ("little", "Little"),
    ("pinky", "Little"),
    ("pinkie", "Little"),
];

/// `ThumbProximal_L` to `thumb`, `proximal`, `l`; `LeftUpperArm` to `left`,
/// `upper`, `arm`; `LHand` to `l`, `hand`; `Spine01` to `spine`, `01`.
pub(super) fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = bare_joint_name(name).chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut word = String::new();
    for (i, c) in chars.iter().copied().enumerate() {
        if !c.is_ascii_alphanumeric() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            continue;
        }
        let prev = i.checked_sub(1).and_then(|p| chars.get(p)).copied();
        let next = chars.get(i + 1).copied();
        let split = prev.is_some_and(|p| {
            (p.is_ascii_lowercase() && c.is_ascii_uppercase())
                || (p.is_ascii_digit() != c.is_ascii_digit() && p.is_ascii_alphanumeric())
                || (p.is_ascii_uppercase()
                    && c.is_ascii_uppercase()
                    && next.is_some_and(|n| n.is_ascii_lowercase()))
        });
        if split && !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
        word.push(c.to_ascii_lowercase());
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn prefix(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// The side the words name; `None` for none or both.
fn side(words: &[String]) -> Option<Side> {
    let left = words.iter().any(|w| w == "left" || w == "l");
    let right = words.iter().any(|w| w == "right" || w == "r");
    match (left, right) {
        (true, false) => Some(Side::Left),
        (false, true) => Some(Side::Right),
        _ => None,
    }
}

/// What one joint's name reads as, before the thumb's naming is known.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Guess {
    /// A VRM bone name.
    Bone(String),
    /// A thumb segment by word (`proximal`), which VRM 0 and VRM 1 count
    /// differently: VRM 0 calls the metacarpal proximal.
    ThumbWord(Side, String),
}

fn has(words: &[String], w: &str) -> bool {
    words.iter().any(|x| x == w)
}

fn finger_guess(words: &[String], side: Side) -> Option<Guess> {
    let (_, finger) = FINGERS.iter().find(|(w, _)| has(words, w))?;
    let digit = words.iter().find_map(|w| w.parse::<u32>().ok());
    let named = ["metacarpal", "proximal", "intermediate", "distal"]
        .into_iter()
        .find(|s| has(words, s));
    let s = side.prefix();
    if *finger == "Thumb" {
        if let Some(word) = named {
            return Some(Guess::ThumbWord(side, word.to_string()));
        }
        let index = usize::try_from(digit?.checked_sub(1)?).ok()?;
        let segment = ["Metacarpal", "Proximal", "Distal"].get(index)?;
        return Some(Guess::Bone(format!("{s}Thumb{segment}")));
    }
    let segment = match (named, digit) {
        (Some("proximal"), _) | (None, Some(1)) => "Proximal",
        (Some("intermediate"), _) | (None, Some(2)) => "Intermediate",
        (Some("distal"), _) | (None, Some(3)) => "Distal",
        _ => return None,
    };
    Some(Guess::Bone(format!("{s}{finger}{segment}")))
}

/// The sided body bone the words name, without its side.
fn limb(words: &[String]) -> Option<&'static str> {
    let any = |list: &[&str]| list.iter().any(|w| has(words, w));
    let upper = any(&["upper", "up"]);
    let lower = any(&["lower", "fore"]);
    let found = if any(&["eye"]) {
        "Eye"
    } else if any(&["toe", "toes", "toebase", "ball"]) {
        "Toes"
    } else if any(&["foot", "ankle"]) {
        "Foot"
    } else if any(&["knee", "calf", "shin", "lowerleg"]) || (lower && has(words, "leg")) {
        "LowerLeg"
    } else if any(&["thigh", "upleg", "upperleg", "leg"]) {
        "UpperLeg"
    } else if any(&["elbow", "forearm", "lowerarm"]) || (lower && has(words, "arm")) {
        "LowerArm"
    } else if any(&["hand", "wrist"]) {
        "Hand"
    } else if any(&["upperarm", "arm"]) || (upper && has(words, "arm")) {
        "UpperArm"
    } else if any(&["shoulder", "clavicle", "collar"]) {
        "Shoulder"
    } else {
        return None;
    };
    Some(found)
}

/// The centre bone the words name.
fn centre(words: &[String]) -> Option<&'static str> {
    let any = |list: &[&str]| list.iter().any(|w| has(words, w));
    let found = if any(&["hips", "hip", "pelvis"]) {
        "hips"
    } else if has(words, "chest") && has(words, "upper") {
        "upperChest"
    } else if has(words, "chest") {
        "chest"
    } else if has(words, "spine") {
        "spine"
    } else if has(words, "neck") {
        "neck"
    } else if has(words, "head") {
        "head"
    } else if has(words, "jaw") {
        "jaw"
    } else {
        return None;
    };
    Some(found)
}

fn guess(name: &str) -> Option<Guess> {
    let words = words(name);
    if words.iter().any(|w| NOT_BONES.contains(&w.as_str())) {
        return None;
    }
    let side = side(&words);
    if FINGERS.iter().any(|(w, _)| has(&words, w)) {
        return finger_guess(&words, side?);
    }
    // A body bone's name carries no number: `Spine1` is a naming's own.
    if words.iter().any(|w| w.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    if let Some(bone) = limb(&words) {
        return Some(Guess::Bone(format!("{}{bone}", side?.prefix())));
    }
    match side {
        None => centre(&words).map(|b| Guess::Bone(b.to_string())),
        Some(_) => None,
    }
}

/// Body words: a joint whose name holds one looks like a humanoid bone,
/// even when its side or joint cannot be read.
const BODY_WORDS: [&str; 30] = [
    "hips", "hip", "pelvis", "spine", "chest", "neck", "head", "shoulder", "clavicle", "arm",
    "forearm", "upperarm", "elbow", "hand", "wrist", "leg", "upleg", "thigh", "knee", "calf",
    "shin", "foot", "ankle", "toe", "thumb", "index", "middle", "ring", "pinky", "little",
];

/// True when the name reads as a body part, helpers aside.
pub(super) fn looks_humanoid(name: &str) -> bool {
    let words = words(name);
    !words.iter().any(|w| NOT_BONES.contains(&w.as_str()))
        && words.iter().any(|w| BODY_WORDS.contains(&w.as_str()))
}

/// Each joint whose name reads as a humanoid bone, with that bone. A side
/// whose thumb has an `intermediate` joint counts its thumb the VRM 0 way.
pub(super) fn guesses(joints: &[(usize, String)]) -> Vec<(usize, String)> {
    let found: Vec<(usize, Guess)> = joints
        .iter()
        .filter_map(|(i, n)| guess(n).map(|g| (*i, g)))
        .collect();
    let old: Vec<Side> = [Side::Left, Side::Right]
        .into_iter()
        .filter(|side| {
            found.iter().any(
                |(_, g)| matches!(g, Guess::ThumbWord(s, w) if s == side && w == "intermediate"),
            )
        })
        .collect();
    let old_thumb = |side: Side| old.contains(&side);
    found
        .into_iter()
        .filter_map(|(i, g)| match g {
            Guess::Bone(b) => Some((i, b)),
            Guess::ThumbWord(side, word) => {
                let segment = match (old_thumb(side), word.as_str()) {
                    (true, "proximal") | (false, "metacarpal") => "Metacarpal",
                    (true, "intermediate") | (false, "proximal") => "Proximal",
                    (_, "distal") => "Distal",
                    _ => return None,
                };
                Some((i, format!("{}Thumb{segment}", side.prefix())))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(name: &str) -> Option<String> {
        guesses(&[(0, name.to_string())]).pop().map(|(_, b)| b)
    }

    #[test]
    fn names_split_into_words() {
        assert_eq!(words("ThumbProximal_L"), ["thumb", "proximal", "l"]);
        assert_eq!(words("LeftUpperArm"), ["left", "upper", "arm"]);
        assert_eq!(words("Upper arm.L"), ["upper", "arm", "l"]);
        assert_eq!(words("LHand"), ["l", "hand"]);
        assert_eq!(words("mixamorig:Spine01"), ["spine", "01"]);
    }

    #[test]
    fn descriptive_names_read_as_bones() {
        for (name, bone) in [
            ("Left leg", "leftUpperLeg"),
            ("Left knee", "leftLowerLeg"),
            ("Right ankle", "rightFoot"),
            ("Left elbow", "leftLowerArm"),
            ("Right shoulder", "rightShoulder"),
            ("Upper arm L", "leftUpperArm"),
            ("Lower_Arm.R", "rightLowerArm"),
            ("wrist_l", "leftHand"),
            ("Left toe", "leftToes"),
            ("Spine", "spine"),
            ("Chest", "chest"),
            ("Upper chest", "upperChest"),
            ("Neck", "neck"),
            ("Head", "head"),
            ("Hips", "hips"),
            ("IndexIntermediate_R", "rightIndexIntermediate"),
            ("Left pinky 3", "leftLittleDistal"),
            ("ThumbProximal_L", "leftThumbProximal"),
        ] {
            assert_eq!(one(name).as_deref(), Some(bone), "{name}");
        }
    }

    #[test]
    fn helpers_and_sideless_limbs_are_not_guessed() {
        for name in [
            "Left arm twist",
            "Head top end",
            "Leg",
            "Spine1",
            "IK target L",
            "Left Right hand",
            "IndexEnd_L",
            "Bone.001",
        ] {
            assert_eq!(one(name), None, "{name}");
        }
    }

    #[test]
    fn a_thumb_with_an_intermediate_joint_counts_the_vrm_0_way() {
        let joints: Vec<(usize, String)> =
            ["ThumbProximal_L", "ThumbIntermediate_L", "ThumbDistal_L"]
                .iter()
                .enumerate()
                .map(|(i, n)| (i, (*n).to_string()))
                .collect();
        let bones: Vec<String> = guesses(&joints).into_iter().map(|(_, b)| b).collect();
        assert_eq!(
            bones,
            [
                "leftThumbMetacarpal",
                "leftThumbProximal",
                "leftThumbDistal"
            ]
        );
    }
}
