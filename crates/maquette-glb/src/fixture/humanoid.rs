//! A box humanoid for the VRM readiness and writer tests: the 65 joint
//! skeleton under any of five namings, with the arms dropped into an
//! A-pose, the whole turned about Y, lifted off the ground, moved off the
//! Y axis, or with float noise in the armature's scale; its skin in any
//! weight format, split over two meshes, or with one known flaw.

use serde_json::json;

pub use super::builder::WeightFormat;
use super::builder::{Builder, Mesh};
use super::pbr_material;
use super::skeleton::{skeleton, world_positions};

mod hands;
mod names;

use hands::{curl_fingers, hair_strip, hand_piece};
use names::rename;

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
    /// Descriptive names a person types: `Left leg`, `Left elbow`,
    /// `Left wrist`, `Upper chest`, `ThumbProximal_L` (VRM 0's finger
    /// names, so the thumb counts from proximal).
    Descriptive,
}

/// One known flaw in the skin or the node tree, in the last box of the
/// body (in the second mesh when the body is split).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SkinFlaw {
    #[default]
    None,
    /// The box's vertices use a joint index one past the skin's joints.
    JointPastEnd,
    /// The box's weights are all zero.
    Unweighted,
    /// The box's weights add up to 0.7.
    OffSum,
    /// The box's vertices have five joints weighted, over two sets: 0.3,
    /// 0.3, 0.2, 0.1 and 0.1, which 8 bit weights round to 1.008.
    FiveInfluences,
    /// The skin lists the hips twice.
    JointTwice,
    /// The left thumb's last joint is also its middle joint's parent.
    Cycle,
    /// The armature's matrix shears.
    Shear,
    /// The left hand sits on the left forearm's joint.
    ShortForearm,
    /// The right pinky's joints have names no naming knows.
    OneSidedFinger,
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
    /// A cape behind the back weighted to the left upper arm, as a rig
    /// that drags cloth with the arm has it.
    pub cape: bool,
    /// A short strip of cloth hanging just below the left upper arm,
    /// weighted to it: close to the arm, but well outside the limb.
    pub cape_strip: bool,
    /// A long strip hanging behind the head down the back, weighted to the
    /// head: a ponytail that moves rigidly with it.
    pub tail: bool,
    pub weights: WeightFormat,
    /// The body is two skinned meshes on one skin, the second with two
    /// primitives.
    pub split: bool,
    pub flaw: SkinFlaw,
    /// Degrees each knuckle and finger joint of the four fingers bends
    /// towards the palm, as an auto rig's curled hand has them.
    pub finger_curl: f32,
    /// A small piece of its own weighted wholly to the left hand, hanging
    /// well off it, as a nail or a held prop an auto rig gives the hand.
    pub hand_piece: bool,
    /// A ribbon of hair sewn to the head that hangs just behind the left
    /// hand, its lower end weighted partly to a finger bone. Built after
    /// every box, so it does not go with `split` or a `flaw`.
    pub hair_strip: bool,
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
            cape: false,
            cape_strip: false,
            tail: false,
            weights: WeightFormat::Float,
            split: false,
            flaw: SkinFlaw::None,
            finger_curl: 0.0,
            hand_piece: false,
            hair_strip: false,
        }
    }
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
    curl_fingers(&joints, &mut world, spec.finger_curl);
    for (side, sign) in [("Left", 1.0f32), ("Right", -1.0f32)] {
        let Some(arm) = joints
            .iter()
            .position(|(n, _, _)| *n == format!("mixamorig:{side}Arm"))
        else {
            continue;
        };
        let pivot = world[arm];
        for (i, p) in world.iter_mut().enumerate() {
            if below(&joints, i, arm) {
                *p = turn_xy(*p, pivot, -sign * spec.arm_drop);
            }
        }
    }
    if spec.flaw == SkinFlaw::ShortForearm
        && let (Some(fore), Some(hand)) = (
            joint_index(&joints, "LeftForeArm"),
            joint_index(&joints, "LeftHand"),
        )
    {
        let shift = [
            world[fore][0] - world[hand][0],
            world[fore][1] - world[hand][1],
            world[fore][2] - world[hand][2],
        ];
        for (i, p) in world.iter_mut().enumerate() {
            if i == hand || below(&joints, i, hand) {
                for (k, d) in shift.iter().enumerate() {
                    p[k] += d;
                }
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
            let name = match bare.strip_prefix("RightHandPinky") {
                Some(n) if spec.flaw == SkinFlaw::OneSidedFinger => format!("RightHandExtra{n}"),
                _ => rename(bare, spec.naming),
            };
            (name, *parent, t)
        })
        .collect()
}

/// The index of a Mixamo joint (without its prefix) in the skeleton.
fn joint_index(joints: &[(String, Option<usize>, [f32; 3])], bare: &str) -> Option<usize> {
    let full = format!("mixamorig:{bare}");
    joints.iter().position(|(n, _, _)| *n == full)
}

/// True when joint `i` sits somewhere under joint `root`.
fn below(joints: &[(String, Option<usize>, [f32; 3])], i: usize, root: usize) -> bool {
    let mut at = joints[i].1;
    while let Some(j) = at {
        if j == root {
            return true;
        }
        at = joints[j].1;
    }
    false
}

/// The slabs of the tail, 2 cm each.
const TAIL_SLABS: usize = 33;

/// A box humanoid built to `spec`.
pub fn humanoid(spec: &HumanoidSpec) -> Vec<u8> {
    let joints = posed(spec);
    let world = world_positions(&joints);
    let at = |i: usize| world[i];
    let mut mesh = Mesh::default();
    // The arm each joint hangs from, with its pivot and how far it dropped,
    // so a box along a dropped arm lies along it rather than square to the
    // world.
    let arms: Vec<(usize, [f32; 3], f32)> = [("LeftArm", 1.0f32), ("RightArm", -1.0f32)]
        .iter()
        .filter_map(|(bare, sign)| {
            let i = joint_index(&skeleton(), bare)?;
            Some((i, at(i), -sign * spec.arm_drop))
        })
        .collect();
    // A box between each joint and its parent, weighted to the parent, so
    // the mesh follows the pose.
    for (i, (_, parent, _)) in joints.iter().enumerate() {
        let Some(p) = *parent else { continue };
        let turn = arms
            .iter()
            .find(|(arm, _, _)| p == *arm || below(&joints, p, *arm))
            .map(|(_, pivot, deg)| (*pivot, *deg));
        let (mut a, mut b) = (at(p), at(i));
        if let Some((pivot, deg)) = turn {
            a = turn_xy(a, pivot, -deg);
            b = turn_xy(b, pivot, -deg);
        }
        let first = mesh.positions.len();
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
        if let Some((pivot, deg)) = turn {
            for v in first..mesh.positions.len() {
                mesh.positions[v] = turn_xy(mesh.positions[v], pivot, deg);
                mesh.normals[v] = turn_xy(mesh.normals[v], [0.0; 3], deg);
            }
        }
    }
    // A sole under each foot, its bottom on the lift height.
    for (i, (name, _, _)) in skeleton().iter().enumerate() {
        if name.ends_with("Foot") {
            let f = at(i);
            mesh.cuboid(
                [f[0], spec.lift + 0.01, f[2] + 0.03],
                [0.08, 0.02, 0.2],
                u16::try_from(i).unwrap_or(0),
            );
        }
    }
    if spec.cape
        && let Some(arm) = skeleton()
            .iter()
            .position(|(n, _, _)| *n == "mixamorig:LeftArm")
    {
        let a = at(arm);
        mesh.cuboid(
            [a[0] / 2.0, a[1] - 0.5, a[2] - 0.25],
            [0.3, 0.5, 0.02],
            u16::try_from(arm).unwrap_or(0),
        );
    }

    if spec.cape_strip
        && let (Some(arm), Some(fore)) = (
            joint_index(&skeleton(), "LeftArm"),
            joint_index(&skeleton(), "LeftForeArm"),
        )
    {
        let (a, b) = (at(arm), at(fore));
        mesh.cuboid(
            [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0 - 0.1, a[2] - 0.03],
            [0.12, 0.05, 0.01],
            u16::try_from(arm).unwrap_or(0),
        );
    }

    if spec.hand_piece {
        hand_piece(&mut mesh, &world);
    }
    if spec.hair_strip {
        hair_strip(&mut mesh, &world);
    }

    if spec.tail
        && let Some(head) = joint_index(&skeleton(), "Head")
    {
        let h = at(head);
        // Thin slabs stacked edge to edge, so the strip reads as one surface
        // hanging from the back of the head to the waist, clear of the
        // thin box body (whose back it would otherwise pass for).
        for k in 0..TAIL_SLABS {
            #[expect(clippy::cast_precision_loss, reason = "a few slabs")]
            let y = h[1] - 0.02 * k as f32;
            mesh.cuboid(
                [h[0], y, h[2] - 0.3],
                [0.1, 0.02, 0.02],
                u16::try_from(head).unwrap_or(0),
            );
        }
    }

    flaw_box(&mut mesh, spec.flaw, joints.len());

    let mut b = Builder::default();
    let (images, textures, materials) = pbr_material(&mut b, 16, "Body");
    let format = spec.weights;
    let mut meshes = vec![];
    if spec.split {
        let half = mesh.boxes() / 2;
        let (first, rest) = mesh.split_boxes(half);
        let (second, third) = rest.split_boxes(rest.boxes() / 2);
        let one = b.primitive_as(&first, 0, true, format);
        let two = [
            b.primitive_as(&second, 0, true, format),
            b.primitive_as(&third, 0, true, format),
        ];
        meshes.push(json!({ "name": "Body", "primitives": [one] }));
        meshes.push(json!({ "name": "Body 2", "primitives": two }));
    } else {
        let prim = b.primitive_as(&mesh, 0, true, format);
        meshes.push(json!({ "name": "Body", "primitives": [prim] }));
    }
    let mut skin_joints: Vec<usize> = (0..joints.len()).collect();
    if spec.flaw == SkinFlaw::JointTwice {
        skin_joints.push(0);
    }
    let ibm: Vec<f32> = skin_joints
        .iter()
        .map(|i| world[*i])
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
    let armature = if spec.flaw == SkinFlaw::Shear {
        json!({
            "name": "Armature",
            "children": [offset],
            "matrix": [1.0, 0.0, 0.0, 0.0, 0.2, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        })
    } else {
        json!({
            "name": "Armature",
            "children": [offset],
            "rotation": [0.0, s, 0.0, c],
            "scale": scale,
        })
    };
    let mut nodes = vec![json!({ "name": "Body", "mesh": 0, "skin": 0 }), armature];
    let looped = (spec.flaw == SkinFlaw::Cycle)
        .then(|| {
            Some((
                joint_index(&skeleton(), "LeftHandThumb4")?,
                joint_index(&skeleton(), "LeftHandThumb2")?,
            ))
        })
        .flatten();
    for (i, (name, _, t)) in joints.iter().enumerate() {
        let mut children: Vec<usize> = joints
            .iter()
            .enumerate()
            .filter(|(_, (_, parent, _))| *parent == Some(i))
            .map(|(c, _)| c + offset)
            .collect();
        if let Some((from, to)) = looped
            && from == i
        {
            children.push(to + offset);
        }
        let mut node = json!({ "name": name, "translation": t });
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        nodes.push(node);
    }
    let mut scene = vec![0, 1];
    if spec.split {
        scene.push(nodes.len());
        nodes.push(json!({ "name": "Body 2", "mesh": 1, "skin": 0 }));
    }
    let mut root = json!({
        "scene": 0,
        "scenes": [{ "nodes": scene }],
        "nodes": nodes,
        "meshes": meshes,
        "skins": [{
            "name": "Armature",
            "inverseBindMatrices": ibm,
            "skeleton": offset,
            "joints": skin_joints.iter().map(|i| i + offset).collect::<Vec<_>>(),
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

/// Puts `flaw` into the last box of `mesh`; `joints` is the skin's count.
fn flaw_box(mesh: &mut Mesh, flaw: SkinFlaw, joints: usize) {
    let n = mesh.positions.len();
    let last = n.saturating_sub(Mesh::BOX_VERTICES)..n;
    let past = u16::try_from(joints).unwrap_or(u16::MAX);
    if flaw == SkinFlaw::FiveInfluences {
        mesh.joints1 = vec![[0; 4]; n];
        mesh.weights1 = vec![[0.0; 4]; n];
    }
    for v in last {
        let j = mesh.joints[v][0];
        match flaw {
            SkinFlaw::JointPastEnd => mesh.joints[v][0] = past,
            SkinFlaw::Unweighted => mesh.weights[v] = [0.0; 4],
            SkinFlaw::OffSum => mesh.weights[v] = [0.7, 0.0, 0.0, 0.0],
            SkinFlaw::FiveInfluences => {
                mesh.joints[v] = [j, 0, 1, 2];
                mesh.weights[v] = [0.3, 0.3, 0.2, 0.1];
                mesh.joints1[v] = [3, 0, 0, 0];
                mesh.weights1[v] = [0.1, 0.0, 0.0, 0.0];
            }
            _ => {}
        }
    }
}
