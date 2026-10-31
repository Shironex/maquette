//! A model's skeleton at rest, from its nodes: each joint's parent and
//! where it stands, in metres and glTF axes, read without Blender. The
//! retarget's rest pose overlay and its bone choices start from it.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::math::{self, IDENTITY, Mat4, Vec3};
use maquette_base::error::{CoreError, CoreResult};

/// One joint at rest. A leaf's tail carries on the way its parent points.
#[derive(Debug, Clone, PartialEq)]
pub struct RestJoint {
    pub name: String,
    pub parent: Option<String>,
    pub head: Vec3,
    pub tail: Vec3,
}

fn walk(node: &gltf::Node<'_>, parent: &Mat4, up: Option<usize>, out: &mut HashMap<usize, Placed>) {
    let world = math::mul(parent, &node.transform().matrix());
    out.insert(
        node.index(),
        Placed {
            head: math::translation(&world),
            up,
        },
    );
    for child in node.children() {
        walk(&child, &world, Some(node.index()), out);
    }
}

struct Placed {
    head: Vec3,
    up: Option<usize>,
}

fn joints_of(doc: &gltf::Document) -> Vec<RestJoint> {
    let mut placed = HashMap::new();
    for scene in doc.scenes() {
        for root in scene.nodes() {
            walk(&root, &IDENTITY, None, &mut placed);
        }
    }
    let mut order: Vec<usize> = Vec::new();
    for skin in doc.skins() {
        for joint in skin.joints() {
            if !order.contains(&joint.index()) {
                order.push(joint.index());
            }
        }
    }
    let nodes: Vec<gltf::Node<'_>> = doc.nodes().collect();
    let name_of = |i: usize| {
        nodes
            .get(i)
            .and_then(|n| n.name())
            .unwrap_or("")
            .to_string()
    };
    let joint_above = |i: usize| {
        let mut at = placed.get(&i)?.up;
        while let Some(up) = at {
            if order.contains(&up) {
                return Some(up);
            }
            at = placed.get(&up)?.up;
        }
        None
    };
    order
        .iter()
        .filter_map(|&i| {
            let head = placed.get(&i)?.head;
            let parent = joint_above(i);
            let child = order.iter().find(|&&c| joint_above(c) == Some(i));
            let tail = match (child, parent) {
                (Some(&c), _) => placed.get(&c)?.head,
                (None, Some(p)) => {
                    let from = placed.get(&p)?.head;
                    [
                        head[0] + (head[0] - from[0]) * 0.5,
                        head[1] + (head[1] - from[1]) * 0.5,
                        head[2] + (head[2] - from[2]) * 0.5,
                    ]
                }
                (None, None) => [head[0], head[1] + 0.1, head[2]],
            };
            Some(RestJoint {
                name: name_of(i),
                parent: parent.map(name_of),
                head,
                tail,
            })
        })
        .collect()
}

/// The joints of every skin in a model file, parents named, at rest.
///
/// # Errors
///
/// Fails when the file cannot be read or is not glTF.
pub fn rest_joints(path: &Path) -> CoreResult<Vec<RestJoint>> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    let gltf = gltf::Gltf::from_slice_without_validation(&bytes)
        .map_err(|e| CoreError::Invalid(format!("This file is not a glTF model: {e}")))?;
    Ok(joints_of(&gltf.document))
}
