//! A box humanoid for the VRM readiness and writer tests: the 65 joint
//! skeleton under any of four namings, with the arms dropped into an
//! A-pose, the whole turned about Y, lifted off the ground, moved off the
//! Y axis, or with float noise in the armature's scale.

use serde_json::json;

use super::builder::{Builder, Mesh};
use super::pbr_material;
use super::skeleton::{skeleton, world_positions};

/// The bone names a fixture humanoid uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Naming {
    /// `mixamorig:LeftArm`.
    Mixamo,
    /// `leftUpperArm`.
    Vrm,
    /// `upperarm_l`.
    Unreal,
    /// `L_Upperarm`, the unverified guess at Tripo's own names.
    Tripo,
}

/// How to build a fixture humanoid. The default is a clean T-pose facing
/// +Z on Mixamo names, with an idle clip.
#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidSpec {
    pub naming: Naming,
    /// Degrees both arms drop below horizontal, about the upper arm.
    pub arm_drop: f32,
    /// Degrees the armature turns about Y; 90 faces +X.
    pub yaw: f32,
    /// Metres the whole body sits above Y = 0.
    pub lift: f32,
    /// Metres the whole body sits off the Y axis before the turn: along
    /// its left (X) and its front (Z).
    pub offset: [f32; 2],
    /// The armature's scale is off 1 by float noise.
    pub scale_noise: bool,
    /// The armature is scaled by this, evenly, when it is not 1.
    pub armature_scale: f32,
    pub animation: bool,
}

impl Default for HumanoidSpec {
    fn default() -> Self {
        HumanoidSpec {
            naming: Naming::Mixamo,
            arm_drop: 0.0,
            yaw: 0.0,
            lift: 0.0,
            offset: [0.0, 0.0],
            scale_noise: false,
            armature_scale: 1.0,
            animation: true,
        }
    }
}

/// The name of a Mixamo joint (without its prefix) under `naming`.
fn rename(bare: &str, naming: Naming) -> String {
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
    }
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

/// Rotates `p` about `pivot` in the XY plane, `deg` degrees (positive turns
/// +X towards +Y).
fn turn_xy(p: [f32; 3], pivot: [f32; 3], deg: f32) -> [f32; 3] {
    let (s, c) = deg.to_radians().sin_cos();
    let (x, y) = (p[0] - pivot[0], p[1] - pivot[1]);
    [pivot[0] + x * c - y * s, pivot[1] + x * s + y * c, p[2]]
}

/// The skeleton's rest positions with the arms dropped and the body lifted.
fn posed(spec: &HumanoidSpec) -> Vec<(String, Option<usize>, [f32; 3])> {
    let joints = skeleton();
    let mut world = world_positions(&joints);
    for (side, sign) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        let Some(arm) = joints
            .iter()
            .position(|(n, _, _)| *n == format!("mixamorig:{side}Arm"))
        else {
            continue;
        };
        let pivot = world[arm];
        let below = |i: usize| {
            let mut at = joints[i].1;
            while let Some(j) = at {
                if j == arm {
                    return true;
                }
                at = joints[j].1;
            }
            false
        };
        for (i, p) in world.iter_mut().enumerate() {
            if below(i) {
                *p = turn_xy(*p, pivot, -sign * spec.arm_drop);
            }
        }
    }
    for p in &mut world {
        p[0] += spec.offset[0];
        p[1] += spec.lift;
        p[2] += spec.offset[1];
    }
    joints
        .iter()
        .enumerate()
        .map(|(i, (name, parent, _))| {
            let base = parent.map_or([0.0; 3], |p| world[p]);
            let t = [
                world[i][0] - base[0],
                world[i][1] - base[1],
                world[i][2] - base[2],
            ];
            let bare = name.trim_start_matches("mixamorig:");
            (rename(bare, spec.naming), *parent, t)
        })
        .collect()
}

/// A box humanoid built to `spec`.
pub fn humanoid(spec: &HumanoidSpec) -> Vec<u8> {
    let joints = posed(spec);
    let world = world_positions(&joints);
    let at = |i: usize| world[i];
    let mut mesh = Mesh::default();
    // A box between each joint and its parent, weighted to the parent, so
    // the mesh follows the pose.
    for (i, (_, parent, _)) in joints.iter().enumerate() {
        let Some(p) = *parent else { continue };
        let (a, b) = (at(p), at(i));
        let centre = [
            (a[0] + b[0]) / 2.0,
            (a[1] + b[1]) / 2.0,
            (a[2] + b[2]) / 2.0,
        ];
        let size = [
            (a[0] - b[0]).abs().max(0.04),
            (a[1] - b[1]).abs().max(0.04),
            (a[2] - b[2]).abs().max(0.04),
        ];
        mesh.cuboid(centre, size, u16::try_from(p).unwrap_or(0));
    }
    // A sole under each foot, its bottom on the lift height.
    for (i, (name, _, _)) in joints.iter().enumerate() {
        if name.to_lowercase().contains("foot") {
            let f = at(i);
            mesh.cuboid(
                [f[0], spec.lift + 0.01, f[2] + 0.03],
                [0.08, 0.02, 0.2],
                u16::try_from(i).unwrap_or(0),
            );
        }
    }

    let mut b = Builder::default();
    let (images, textures, materials) = pbr_material(&mut b, 16, "Body");
    let prim = b.primitive(&mesh, 0, true);
    let ibm: Vec<f32> = world
        .iter()
        .flat_map(|p| {
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -p[0], -p[1], -p[2],
                1.0,
            ]
        })
        .collect();
    let ibm = b.floats(&ibm, "MAT4", 16, None, false);
    let offset = 2;
    let (s, c) = (spec.yaw.to_radians() / 2.0).sin_cos();
    let scale = if spec.scale_noise {
        [1.000_000_1, 0.999_999_8, 0.999_999_9]
    } else {
        [spec.armature_scale; 3]
    };
    let mut nodes = vec![
        json!({ "name": "Body", "mesh": 0, "skin": 0 }),
        json!({
            "name": "Armature",
            "children": [offset],
            "rotation": [0.0, s, 0.0, c],
            "scale": scale,
        }),
    ];
    for (i, (name, _, t)) in joints.iter().enumerate() {
        let children: Vec<usize> = joints
            .iter()
            .enumerate()
            .filter(|(_, (_, parent, _))| *parent == Some(i))
            .map(|(c, _)| c + offset)
            .collect();
        let mut node = json!({ "name": name, "translation": t });
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        nodes.push(node);
    }
    let mut root = json!({
        "scene": 0,
        "scenes": [{ "nodes": [0, 1] }],
        "nodes": nodes,
        "meshes": [{ "name": "Body", "primitives": [prim] }],
        "skins": [{
            "name": "Armature",
            "inverseBindMatrices": ibm,
            "skeleton": offset,
            "joints": (0..joints.len()).map(|i| i + offset).collect::<Vec<_>>(),
        }],
        "materials": materials,
        "images": images,
        "textures": textures,
        "samplers": [{}],
    });
    if spec.animation {
        let times = b.floats(&[0.0, 1.0], "SCALAR", 1, None, true);
        let hips = joints[0].2;
        let values = b.floats(
            &[hips[0], hips[1], hips[2], hips[0], hips[1] - 0.02, hips[2]],
            "VEC3",
            3,
            None,
            false,
        );
        root["animations"] = json!([{
            "name": "Idle",
            "samplers": [{ "input": times, "output": values, "interpolation": "LINEAR" }],
            "channels": [{ "sampler": 0, "target": { "node": offset, "path": "translation" } }],
        }]);
    }
    b.finish(root)
}
