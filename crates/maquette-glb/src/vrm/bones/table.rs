//! The table of VRM humanoid bones: names, parents, whether VRM requires
//! them, and the names each naming gives them.

use super::BoneNaming;
use crate::analyse::bare_joint_name;

/// One humanoid bone with its sides filled in.
#[derive(Debug, Clone)]
pub(crate) struct HumanBone {
    /// The VRM name: `hips`, `leftUpperArm`.
    pub(crate) name: String,
    pub(crate) parent: Option<String>,
    pub(crate) required: bool,
    /// Normalised names per naming.
    pub(in crate::vrm) names: Vec<(BoneNaming, String)>,
}

/// A bone before its side is filled in: `{side}` reads `left` or `right`,
/// `{s}` reads `l` or `r`. Case does not matter when names are compared.
struct Def {
    name: &'static str,
    parent: Option<&'static str>,
    required: bool,
    /// Mixamo, Unreal, Tripo and VRoid names, in that order.
    names: [&'static [&'static str]; 4],
}

const fn def(
    name: &'static str,
    parent: Option<&'static str>,
    required: bool,
    names: [&'static [&'static str]; 4],
) -> Def {
    Def {
        name,
        parent,
        required,
        names,
    }
}

/// The torso, head, legs and arms; fingers come from [`fingers`]. Names in
/// order: Mixamo, Unreal, Tripo, VRoid.
const BODY: [Def; 16] = [
    def(
        "hips",
        None,
        true,
        [&["Hips"], &["pelvis"], &["Hip"], &["J_Bip_C_Hips"]],
    ),
    def(
        "spine",
        Some("hips"),
        true,
        [&["Spine"], &["spine_01"], &["Waist"], &["J_Bip_C_Spine"]],
    ),
    def(
        "chest",
        Some("spine"),
        false,
        [&["Spine1"], &["spine_02"], &["Spine01"], &["J_Bip_C_Chest"]],
    ),
    def(
        "upperChest",
        Some("chest"),
        false,
        [
            &["Spine2"],
            &["spine_03"],
            &["Spine02"],
            &["J_Bip_C_UpperChest"],
        ],
    ),
    def(
        "neck",
        Some("upperChest"),
        false,
        [&["Neck"], &["neck_01"], &["NeckTwist01"], &["J_Bip_C_Neck"]],
    ),
    def(
        "head",
        Some("neck"),
        true,
        [&["Head"], &["head"], &["Head"], &["J_Bip_C_Head"]],
    ),
    def(
        "{side}Eye",
        Some("head"),
        false,
        [&["{side}Eye"], &["eye_{s}"], &[], &["J_Adj_{s}_FaceEye"]],
    ),
    def("jaw", Some("head"), false, [&[], &["jaw"], &[], &[]]),
    def(
        "{side}UpperLeg",
        Some("hips"),
        true,
        [
            &["{side}UpLeg"],
            &["thigh_{s}"],
            &["{s}_Thigh"],
            &["J_Bip_{s}_UpperLeg"],
        ],
    ),
    def(
        "{side}LowerLeg",
        Some("{side}UpperLeg"),
        true,
        [
            &["{side}Leg"],
            &["calf_{s}"],
            &["{s}_Calf"],
            &["J_Bip_{s}_LowerLeg"],
        ],
    ),
    def(
        "{side}Foot",
        Some("{side}LowerLeg"),
        true,
        [
            &["{side}Foot"],
            &["foot_{s}"],
            &["{s}_Foot"],
            &["J_Bip_{s}_Foot"],
        ],
    ),
    def(
        "{side}Toes",
        Some("{side}Foot"),
        false,
        [
            &["{side}ToeBase"],
            &["ball_{s}"],
            &["{s}_ToeBase"],
            &["J_Bip_{s}_ToeBase"],
        ],
    ),
    def(
        "{side}Shoulder",
        Some("upperChest"),
        false,
        [
            &["{side}Shoulder"],
            &["clavicle_{s}"],
            &["{s}_Clavicle"],
            &["J_Bip_{s}_Shoulder"],
        ],
    ),
    def(
        "{side}UpperArm",
        Some("{side}Shoulder"),
        true,
        [
            &["{side}Arm"],
            &["upperarm_{s}"],
            &["{s}_Upperarm"],
            &["J_Bip_{s}_UpperArm"],
        ],
    ),
    def(
        "{side}LowerArm",
        Some("{side}UpperArm"),
        true,
        [
            &["{side}ForeArm"],
            &["lowerarm_{s}"],
            &["{s}_Forearm"],
            &["J_Bip_{s}_LowerArm"],
        ],
    ),
    def(
        "{side}Hand",
        Some("{side}LowerArm"),
        true,
        [
            &["{side}Hand"],
            &["hand_{s}"],
            &["{s}_Hand"],
            &["J_Bip_{s}_Hand"],
        ],
    ),
];

/// The VRM finger, the segments VRM names in order, and the Mixamo, Unreal
/// and VRoid finger names (their joints are numbered 1 to 3).
const FINGERS: [(&str, [&str; 3], &str, &str, &str); 5] = [
    (
        "Thumb",
        ["Metacarpal", "Proximal", "Distal"],
        "Thumb",
        "thumb",
        "Thumb",
    ),
    (
        "Index",
        ["Proximal", "Intermediate", "Distal"],
        "Index",
        "index",
        "Index",
    ),
    (
        "Middle",
        ["Proximal", "Intermediate", "Distal"],
        "Middle",
        "middle",
        "Middle",
    ),
    (
        "Ring",
        ["Proximal", "Intermediate", "Distal"],
        "Ring",
        "ring",
        "Ring",
    ),
    (
        "Little",
        ["Proximal", "Intermediate", "Distal"],
        "Pinky",
        "pinky",
        "Little",
    ),
];

/// How many humanoid bones VRM 1.0 names.
pub const HUMAN_BONES: usize = 55;

/// How many of them VRM requires.
pub const REQUIRED_BONES: usize = 15;

/// `upper_arm.L` and `Upper-Arm_L` both read `upper_arm_l`.
pub(crate) fn normal(name: &str) -> String {
    bare_joint_name(name)
        .chars()
        .map(|c| match c {
            '.' | '-' | ' ' => '_',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

fn fill(template: &str, side: Option<(&str, &str)>) -> String {
    match side {
        Some((word, letter)) => template.replace("{side}", word).replace("{s}", letter),
        None => template.to_string(),
    }
}

fn sides(template: &str) -> Vec<Option<(&'static str, &'static str)>> {
    if template.contains("{side}") {
        vec![Some(("left", "l")), Some(("right", "r"))]
    } else {
        vec![None]
    }
}

/// Every humanoid bone, parents before children: the body, then each
/// side's fingers.
pub(crate) fn human_bones() -> Vec<HumanBone> {
    let mut out = Vec::with_capacity(HUMAN_BONES);
    for d in &BODY {
        let [mixamo, unreal, tripo, vroid] = d.names;
        for side in sides(d.name) {
            let mut names = vec![(BoneNaming::Vrm, normal(&fill(d.name, side)))];
            for (naming, list) in [
                (BoneNaming::Vrm, vroid),
                (BoneNaming::Mixamo, mixamo),
                (BoneNaming::Unreal, unreal),
                (BoneNaming::Tripo, tripo),
            ] {
                names.extend(list.iter().map(|n| (naming, normal(&fill(n, side)))));
            }
            out.push(HumanBone {
                name: fill(d.name, side),
                parent: d.parent.map(|p| fill(p, side)),
                required: d.required,
                names,
            });
        }
    }
    for (word, letter) in [("left", "l"), ("right", "r")] {
        let upper = if word == "left" { "Left" } else { "Right" };
        for (finger, segments, mixamo, unreal, vroid) in FINGERS {
            let mut parent = format!("{word}Hand");
            for (i, segment) in segments.iter().enumerate() {
                let n = i + 1;
                let name = format!("{word}{finger}{segment}");
                let names = vec![
                    (BoneNaming::Vrm, normal(&name)),
                    (
                        BoneNaming::Vrm,
                        normal(&format!("J_Bip_{letter}_{vroid}{n}")),
                    ),
                    (
                        BoneNaming::Mixamo,
                        normal(&format!("{upper}Hand{mixamo}{n}")),
                    ),
                    (
                        BoneNaming::Unreal,
                        normal(&format!("{unreal}_0{n}_{letter}")),
                    ),
                ];
                out.push(HumanBone {
                    name: name.clone(),
                    parent: Some(std::mem::replace(&mut parent, name)),
                    required: false,
                    names,
                });
            }
        }
    }
    out
}

/// A bone VRM needs set before this one may be: the upper chest needs the
/// chest, and a finger segment the one before it.
pub(crate) fn needs(bone: &HumanBone) -> Option<&str> {
    let parent = bone.parent.as_deref()?;
    let finger = ["Metacarpal", "Proximal", "Intermediate"]
        .iter()
        .any(|s| parent.ends_with(s));
    (bone.name == "upperChest" || finger).then_some(parent)
}
