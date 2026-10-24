//! Reads a GLB or glTF file and measures what the inbox checks need:
//! counts, texture sizes, the rig, and two geometry heuristics.
//!
//! **Merged objects.** Within one primitive, vertices at the same position
//! are welded and triangles are joined into connected islands. Islands
//! whose boxes sit within [`GROUP_GAP`] of the primitive's diagonal of each
//! other are one object (a lid resting on a crate is still one crate); two
//! or more groups that each hold at least [`GROUP_MIN_SHARE`] of the
//! triangles look like separate objects that were joined into one mesh.
//!
//! **Hair weighted to finger bones.** For every skinned vertex, the joint
//! with the largest weight is its dominant joint. When that joint is a hand
//! or finger bone (Mixamo names) but the vertex sits more than
//! [`FINGER_REACH`] of the model height from the joint's bind position, or
//! above the head joint, it is flagged: long hair and tails near the hands
//! often get auto weighted to the fingers and then fly around with them.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use gltf::buffer::Source as BufferSource;
use gltf::image::Source as ImageSource;
use gltf::mesh::Mode;
use serde::{Deserialize, Serialize};

use super::images;
use super::math::{self, Aabb, IDENTITY, Mat4, Vec3};
use crate::error::{CoreError, CoreResult};

/// Islands closer than this share of the primitive's diagonal are grouped.
pub const GROUP_GAP: f32 = 0.1;
/// A group needs this share of the primitive's triangles to count.
pub const GROUP_MIN_SHARE: f32 = 0.01;
/// How far, as a share of model height, a vertex may sit from its hand or
/// finger joint before it is flagged.
pub const FINGER_REACH: f32 = 0.15;
/// Islands considered per primitive, largest first, to bound the work.
const MAX_ISLANDS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RigKind {
    None,
    Mixamo,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextureInfo {
    pub name: String,
    pub mime: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bytes: f64,
    /// Material slots that use it: "base color", "normal" and so on.
    pub uses: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PbrMaps {
    pub base_color: bool,
    pub metallic_roughness: bool,
    pub normal: bool,
    pub occlusion: bool,
    pub emissive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct JointCount {
    pub name: String,
    pub vertices: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FingerWeights {
    pub vertices: u32,
    /// The finger and hand joints holding flagged vertices, most first.
    pub joints: Vec<JointCount>,
    /// The distance used, in model units.
    pub reach: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Islands {
    /// The primitive with the most groups.
    pub mesh: String,
    pub islands: u32,
    pub groups: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GlbReport {
    pub bytes: f64,
    pub generator: Option<String>,
    pub triangles: u32,
    pub vertices: u32,
    pub meshes: u32,
    pub primitives: u32,
    pub materials: u32,
    pub textures: Vec<TextureInfo>,
    /// The longest texture edge, 0 without textures.
    pub max_texture: u32,
    pub skins: u32,
    pub joints: u32,
    pub joint_names: Vec<String>,
    pub rig: RigKind,
    pub animations: Vec<String>,
    /// Width, height and depth of the default scene.
    pub size: [f32; 3],
    pub pbr: PbrMaps,
    pub extensions: Vec<String>,
    /// False for meshopt or Draco compressed geometry, where the heuristics
    /// below are skipped.
    pub geometry_readable: bool,
    pub islands: Option<Islands>,
    pub finger_weights: Option<FingerWeights>,
}

/// Analyses the model at `path`.
///
/// # Errors
///
/// Fails when the file cannot be read or is not glTF.
pub fn analyse(path: &Path) -> CoreResult<GlbReport> {
    let bytes = fs::read(path).map_err(|e| CoreError::io(path, e))?;
    let base = path.parent().unwrap_or(Path::new("."));
    // The parser trusts indices it has not validated and can panic on a
    // malformed file; that is a bad file, not a crash.
    catch_unwind(AssertUnwindSafe(|| analyse_bytes(&bytes, base))).unwrap_or_else(|_| {
        Err(CoreError::Invalid(
            "This file is not a well formed glTF model.".to_string(),
        ))
    })
}

/// Analyses model bytes; external files resolve against `base`.
///
/// # Errors
///
/// Fails when the bytes are not glTF.
pub fn analyse_bytes(bytes: &[u8], base: &Path) -> CoreResult<GlbReport> {
    let gltf = gltf::Gltf::from_slice_without_validation(bytes)
        .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
    let doc = &gltf.document;
    let buffers: Vec<Option<Vec<u8>>> = doc
        .buffers()
        .map(|buffer| match buffer.source() {
            BufferSource::Bin => gltf.blob.clone(),
            BufferSource::Uri(uri) => load_uri(uri, base),
        })
        .collect();
    let extensions: Vec<String> = doc.extensions_used().map(str::to_string).collect();
    let compressed = extensions.iter().any(|e| {
        e == "EXT_meshopt_compression"
            || e == "KHR_meshopt_compression"
            || e == "KHR_draco_mesh_compression"
    });

    let worlds = world_matrices(doc);
    let mut bounds = Aabb::EMPTY;
    let (mut triangles, mut vertices, mut primitives) = (0u64, 0u64, 0u32);
    let mut islands: Option<Islands> = None;
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        let world = worlds.get(node.index()).copied().unwrap_or(IDENTITY);
        for prim in mesh.primitives() {
            primitives += 1;
            let count = prim
                .get(&gltf::Semantic::Positions)
                .map_or(0, |a| a.count()) as u64;
            vertices += count;
            let elements = prim.indices().map_or(count, |a| a.count() as u64);
            triangles += match prim.mode() {
                Mode::Triangles => elements / 3,
                Mode::TriangleStrip | Mode::TriangleFan => elements.saturating_sub(2),
                _ => 0,
            };
            let bb = prim.bounding_box();
            for corner in corners(bb.min, bb.max) {
                bounds.add(math::point(&world, corner));
            }
            if !compressed && prim.mode() == Mode::Triangles {
                let name = mesh.name().or(node.name()).unwrap_or("mesh").to_string();
                if let Some(found) = primitive_islands(&prim, &buffers, &name)
                    && islands.as_ref().is_none_or(|i| found.groups > i.groups)
                {
                    islands = Some(found);
                }
            }
        }
    }

    let (textures, pbr) = textures(doc, &buffers, base);
    let max_texture = textures
        .iter()
        .map(|t| t.width.unwrap_or(0).max(t.height.unwrap_or(0)))
        .max()
        .unwrap_or(0);
    let joint_names: Vec<String> = {
        let mut seen = Vec::new();
        for skin in doc.skins() {
            for joint in skin.joints() {
                let name = joint.name().unwrap_or("joint").to_string();
                if !seen.contains(&name) {
                    seen.push(name);
                }
            }
        }
        seen
    };
    let rig = rig_kind(&joint_names);
    let finger_weights = if compressed {
        None
    } else {
        finger_weights(doc, &buffers, &worlds)
    };

    Ok(GlbReport {
        bytes: bytes.len() as f64,
        generator: doc_generator(&gltf),
        triangles: u32::try_from(triangles).unwrap_or(u32::MAX),
        vertices: u32::try_from(vertices).unwrap_or(u32::MAX),
        meshes: count(doc.meshes().len()),
        primitives,
        materials: count(doc.materials().len()),
        max_texture,
        textures,
        skins: count(doc.skins().len()),
        joints: count(joint_names.len()),
        joint_names,
        rig,
        animations: doc
            .animations()
            .enumerate()
            .map(|(i, a)| {
                a.name()
                    .map_or_else(|| format!("Animation {}", i + 1), str::to_string)
            })
            .collect(),
        size: bounds.size(),
        pbr,
        extensions,
        geometry_readable: !compressed,
        islands,
        finger_weights,
    })
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn doc_generator(gltf: &gltf::Gltf) -> Option<String> {
    gltf.document.as_json().asset.generator.clone()
}

fn corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    let mut out = [[0.0; 3]; 8];
    for (i, c) in out.iter_mut().enumerate() {
        *c = [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ];
    }
    out
}

/// World matrices of every node, from the scene roots down.
pub(crate) fn world_matrices(doc: &gltf::Document) -> Vec<Mat4> {
    let mut out = vec![IDENTITY; doc.nodes().len()];
    let mut has_parent = vec![false; doc.nodes().len()];
    for node in doc.nodes() {
        for child in node.children() {
            if let Some(flag) = has_parent.get_mut(child.index()) {
                *flag = true;
            }
        }
    }
    let mut stack: Vec<(gltf::Node<'_>, Mat4)> = doc
        .nodes()
        .filter(|n| !has_parent[n.index()])
        .map(|n| (n, IDENTITY))
        .collect();
    let mut visited = vec![false; doc.nodes().len()];
    while let Some((node, parent)) = stack.pop() {
        if std::mem::replace(&mut visited[node.index()], true) {
            continue;
        }
        let world = math::mul(&parent, &node.transform().matrix());
        out[node.index()] = world;
        for child in node.children() {
            stack.push((child, world));
        }
    }
    out
}

fn load_uri(uri: &str, base: &Path) -> Option<Vec<u8>> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (_, data) = rest.split_once(";base64,")?;
        return base64_decode(data);
    }
    if uri.contains("://") {
        return None;
    }
    let decoded = percent_decode(uri);
    let path = base.join(decoded);
    // External files must sit next to the model, never above it.
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    fs::read(path).ok()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = s.get(i + 1..i + 3)
            && let Ok(v) = u8::from_str_radix(hex, 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(u32::from(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        }))
    };
    let clean: Vec<u8> = s
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= value(c)? << (18 - 6 * i);
        }
        let bytes = acc.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    Some(out)
}

fn textures(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    base: &Path,
) -> (Vec<TextureInfo>, PbrMaps) {
    // Image index for a texture, including KTX2 sources behind
    // KHR_texture_basisu.
    let image_of = |texture: &gltf::Texture<'_>| -> Option<usize> {
        texture.source().map(|i| i.index()).or_else(|| {
            texture
                .extensions()?
                .get("KHR_texture_basisu")?
                .get("source")?
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
        })
    };
    let mut uses: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut pbr = PbrMaps::default();
    for material in doc.materials() {
        let m = material.pbr_metallic_roughness();
        let slots = [
            ("base color", m.base_color_texture().map(|t| t.texture())),
            (
                "metal and roughness",
                m.metallic_roughness_texture().map(|t| t.texture()),
            ),
            ("normal", material.normal_texture().map(|t| t.texture())),
            (
                "occlusion",
                material.occlusion_texture().map(|t| t.texture()),
            ),
            ("emissive", material.emissive_texture().map(|t| t.texture())),
        ];
        for (slot, texture) in slots {
            let Some(texture) = texture else { continue };
            match slot {
                "base color" => pbr.base_color = true,
                "metal and roughness" => pbr.metallic_roughness = true,
                "normal" => pbr.normal = true,
                "occlusion" => pbr.occlusion = true,
                _ => pbr.emissive = true,
            }
            if let Some(image) = image_of(&texture) {
                let list = uses.entry(image).or_default();
                if !list.iter().any(|s| s == slot) {
                    list.push(slot.to_string());
                }
            }
        }
    }
    let list = doc
        .images()
        .map(|image| {
            let (data, mime) = match image.source() {
                ImageSource::View { view, mime_type } => {
                    let data = buffers
                        .get(view.buffer().index())
                        .and_then(|b| b.as_deref())
                        .and_then(|b| b.get(view.offset()..view.offset() + view.length()))
                        .map(<[u8]>::to_vec);
                    (data, Some(mime_type.to_string()))
                }
                ImageSource::Uri { uri, mime_type } => {
                    (load_uri(uri, base), mime_type.map(str::to_string))
                }
            };
            let dims = data.as_deref().and_then(images::dimensions);
            let mime = mime.or_else(|| data.as_deref().and_then(images::sniff).map(str::to_string));
            TextureInfo {
                name: image
                    .name()
                    .map_or_else(|| format!("Image {}", image.index() + 1), str::to_string),
                mime,
                width: dims.map(|d| d.0),
                height: dims.map(|d| d.1),
                bytes: data.as_ref().map_or(0.0, |d| d.len() as f64),
                uses: uses.remove(&image.index()).unwrap_or_default(),
            }
        })
        .collect();
    (list, pbr)
}

/// The name without a rig prefix such as `mixamorig:` or `mixamorig1_`.
pub fn bare_joint_name(name: &str) -> &str {
    let after = name.rsplit(':').next().unwrap_or(name);
    let lower = after.to_ascii_lowercase();
    if lower.starts_with("mixamorig")
        && let Some(pos) = after.find('_')
    {
        return &after[pos + 1..];
    }
    after
}

const MIXAMO_CORE: [&str; 20] = [
    "Hips",
    "Spine",
    "Spine1",
    "Spine2",
    "Neck",
    "Head",
    "LeftShoulder",
    "LeftArm",
    "LeftForeArm",
    "LeftHand",
    "RightShoulder",
    "RightArm",
    "RightForeArm",
    "RightHand",
    "LeftUpLeg",
    "LeftLeg",
    "LeftFoot",
    "RightUpLeg",
    "RightLeg",
    "RightFoot",
];

fn rig_kind(joints: &[String]) -> RigKind {
    if joints.is_empty() {
        return RigKind::None;
    }
    let found = MIXAMO_CORE
        .iter()
        .filter(|core| joints.iter().any(|j| bare_joint_name(j) == **core))
        .count();
    if found >= 18 {
        RigKind::Mixamo
    } else {
        RigKind::Other
    }
}

/// A hand or finger joint in Mixamo naming.
pub fn is_hand_joint(name: &str) -> bool {
    let bare = bare_joint_name(name);
    bare.starts_with("LeftHand") || bare.starts_with("RightHand")
}

struct UnionFind(Vec<u32>);

impl UnionFind {
    fn new(n: usize) -> Self {
        Self(
            (0..n)
                .map(|i| u32::try_from(i).unwrap_or(u32::MAX))
                .collect(),
        )
    }

    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            let parent = self.0[x as usize];
            self.0[x as usize] = self.0[parent as usize];
            x = parent;
        }
        x
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[ra as usize] = rb;
        }
    }
}

fn primitive_islands(
    prim: &gltf::Primitive<'_>,
    buffers: &[Option<Vec<u8>>],
    name: &str,
) -> Option<Islands> {
    let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
    let positions: Vec<Vec3> = reader.read_positions()?.collect();
    if positions.len() < 3 {
        return None;
    }
    let indices: Vec<u32> = match reader.read_indices() {
        Some(i) => i.into_u32().collect(),
        None => (0..u32::try_from(positions.len()).ok()?).collect(),
    };
    let mut bounds = Aabb::EMPTY;
    for p in &positions {
        bounds.add(*p);
    }
    let diagonal = bounds.diagonal().max(1e-6);
    // Weld vertices that share a position (UV and normal seams split them).
    let cell = diagonal * 1e-5;
    let mut welded: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let ids: Vec<u32> = positions
        .iter()
        .map(|p| {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "grid cells of a bounded mesh"
            )]
            let key = (
                (p[0] / cell).round() as i64,
                (p[1] / cell).round() as i64,
                (p[2] / cell).round() as i64,
            );
            let next = u32::try_from(welded.len()).unwrap_or(u32::MAX);
            *welded.entry(key).or_insert(next)
        })
        .collect();
    let mut sets = UnionFind::new(welded.len());
    let triangles: Vec<[u32; 3]> = indices
        .chunks_exact(3)
        .filter_map(|t| {
            let v = [
                *ids.get(t[0] as usize)?,
                *ids.get(t[1] as usize)?,
                *ids.get(t[2] as usize)?,
            ];
            Some(v)
        })
        .collect();
    for t in &triangles {
        sets.union(t[0], t[1]);
        sets.union(t[1], t[2]);
    }
    let mut islands: HashMap<u32, (Aabb, u32)> = HashMap::new();
    for (tri, t) in triangles.iter().zip(indices.chunks_exact(3)) {
        let root = sets.find(tri[0]);
        let entry = islands.entry(root).or_insert((Aabb::EMPTY, 0));
        for &i in t {
            if let Some(p) = positions.get(i as usize) {
                entry.0.add(*p);
            }
        }
        entry.1 += 1;
    }
    let island_count = count(islands.len());
    let mut list: Vec<(Aabb, u32)> = islands.into_values().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1));
    list.truncate(MAX_ISLANDS);
    let mut groups = UnionFind::new(list.len());
    let gap = diagonal * GROUP_GAP;
    for i in 0..list.len() {
        for j in i + 1..list.len() {
            if list[i].0.gap(&list[j].0) <= gap {
                groups.union(count(i), count(j));
            }
        }
    }
    let mut tris_per_group: HashMap<u32, u32> = HashMap::new();
    for (i, island) in list.iter().enumerate() {
        *tris_per_group.entry(groups.find(count(i))).or_default() += island.1;
    }
    let total = count(triangles.len()).max(1) as f32;
    let significant = tris_per_group
        .values()
        .filter(|&&t| t as f32 / total >= GROUP_MIN_SHARE)
        .count();
    Some(Islands {
        mesh: name.to_string(),
        islands: island_count,
        groups: count(significant),
    })
}

fn finger_weights(
    doc: &gltf::Document,
    buffers: &[Option<Vec<u8>>],
    worlds: &[Mat4],
) -> Option<FingerWeights> {
    let mut flagged = 0u32;
    let mut per_joint: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = false;
    let mut reach_used = 0.0f32;
    for node in doc.nodes() {
        let (Some(mesh), Some(skin)) = (node.mesh(), node.skin()) else {
            continue;
        };
        let joints: Vec<gltf::Node<'_>> = skin.joints().collect();
        let ibms: Vec<Mat4> = skin
            .reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()))
            .read_inverse_bind_matrices()
            .map(Iterator::collect)
            .unwrap_or_default();
        // Bind position of each joint in the mesh's space.
        let bind: Vec<Vec3> = joints
            .iter()
            .enumerate()
            .map(|(i, joint)| {
                ibms.get(i)
                    .and_then(math::invert_affine)
                    .map(|m| math::translation(&m))
                    .unwrap_or_else(|| {
                        math::translation(&worlds.get(joint.index()).copied().unwrap_or(IDENTITY))
                    })
            })
            .collect();
        let names: Vec<String> = joints
            .iter()
            .map(|j| j.name().unwrap_or("joint").to_string())
            .collect();
        let head_y = names
            .iter()
            .position(|n| bare_joint_name(n) == "Head")
            .and_then(|i| bind.get(i))
            .map(|p| p[1]);
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers.get(b.index()).and_then(|x| x.as_deref()));
            let (Some(positions), Some(joint_sets), Some(weights)) = (
                reader.read_positions(),
                reader.read_joints(0),
                reader.read_weights(0),
            ) else {
                continue;
            };
            let positions: Vec<Vec3> = positions.collect();
            let mut bounds = Aabb::EMPTY;
            for p in &positions {
                bounds.add(*p);
            }
            let reach = bounds.size()[1].max(1e-6) * FINGER_REACH;
            reach_used = reach_used.max(reach);
            checked = true;
            for ((p, j), w) in positions
                .iter()
                .zip(joint_sets.into_u16())
                .zip(weights.into_f32())
            {
                let (k, _) = w.iter().enumerate().fold((0, f32::MIN), |best, (k, &v)| {
                    if v > best.1 { (k, v) } else { best }
                });
                let joint = usize::from(j[k]);
                let (Some(name), Some(at)) = (names.get(joint), bind.get(joint)) else {
                    continue;
                };
                if !is_hand_joint(name) {
                    continue;
                }
                let far = math::distance(*p, *at) > reach;
                let above_head = head_y.is_some_and(|y| p[1] > y);
                if far || above_head {
                    flagged += 1;
                    *per_joint.entry(name.clone()).or_default() += 1;
                }
            }
        }
    }
    if !checked {
        return None;
    }
    let mut joints: Vec<JointCount> = per_joint
        .into_iter()
        .map(|(name, vertices)| JointCount { name, vertices })
        .collect();
    joints.sort_by(|a, b| b.vertices.cmp(&a.vertices).then(a.name.cmp(&b.name)));
    Some(FingerWeights {
        vertices: flagged,
        joints,
        reach: reach_used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbox::fixture;

    fn report(bytes: &[u8]) -> GlbReport {
        analyse_bytes(bytes, Path::new(".")).expect("analyse")
    }

    #[test]
    fn a_clean_prop_measures_cleanly() {
        let r = report(&fixture::clean_prop());
        assert_eq!(
            (r.triangles, r.vertices, r.meshes, r.primitives, r.materials),
            (12, 24, 1, 1, 1)
        );
        assert_eq!(r.textures.len(), 3);
        assert_eq!(r.max_texture, 64);
        assert_eq!(r.textures[0].uses, vec!["base color".to_string()]);
        assert_eq!(r.textures[0].mime.as_deref(), Some("image/png"));
        assert!(r.pbr.base_color && r.pbr.metallic_roughness && r.pbr.normal);
        assert!(!r.pbr.occlusion);
        assert_eq!(r.rig, RigKind::None);
        assert!(r.animations.is_empty());
        assert!((r.size[1] - 0.5).abs() < 1e-5);
        let islands = r.islands.expect("islands");
        assert_eq!((islands.islands, islands.groups), (1, 1));
        assert!(r.finger_weights.is_none());
        assert_eq!(r.generator.as_deref(), Some("Maquette fixtures"));
    }

    #[test]
    fn merged_objects_show_as_separate_groups() {
        let r = report(&fixture::merged_islands());
        let islands = r.islands.expect("islands");
        assert_eq!(islands.islands, 3, "three boxes");
        assert_eq!(islands.groups, 2, "the touching pair is one object");
        assert_eq!(islands.mesh, "Props");
        assert!(!r.pbr.base_color);
    }

    #[test]
    fn tails_weighted_to_fingers_are_found() {
        let r = report(&fixture::character(true));
        assert_eq!(r.rig, RigKind::Mixamo);
        assert_eq!(r.joints, 65);
        assert_eq!(r.skins, 1);
        assert_eq!(r.animations, vec!["Idle".to_string()]);
        let fw = r.finger_weights.expect("checked");
        // Two tails of 24 vertices each.
        assert_eq!(fw.vertices, 48);
        let names: Vec<&str> = fw.joints.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(
            names,
            ["mixamorig:LeftHandIndex1", "mixamorig:RightHandIndex1"]
        );
        assert!(fw.joints.iter().all(|j| j.vertices == 24));
        // The real finger boxes stay unflagged.
        let islands = r.islands.expect("islands");
        assert_eq!(islands.groups, 1, "a character is one object");

        let clean = report(&fixture::character(false));
        assert_eq!(clean.finger_weights.expect("checked").vertices, 0);
    }

    #[test]
    fn joint_names_and_rig_detection() {
        assert_eq!(
            bare_joint_name("mixamorig:LeftHandIndex1"),
            "LeftHandIndex1"
        );
        assert_eq!(bare_joint_name("mixamorig1_LeftHand"), "LeftHand");
        assert_eq!(bare_joint_name("Spine"), "Spine");
        assert!(is_hand_joint("mixamorig:RightHandPinky3"));
        assert!(is_hand_joint("LeftHand"));
        assert!(!is_hand_joint("mixamorig:LeftForeArm"));
        let names: Vec<String> = MIXAMO_CORE.iter().map(|n| n.to_string()).collect();
        assert_eq!(rig_kind(&names), RigKind::Mixamo);
        assert_eq!(
            rig_kind(&["Bone".to_string(), "Bone.001".to_string()]),
            RigKind::Other
        );
        assert_eq!(rig_kind(&[]), RigKind::None);
    }

    #[test]
    fn files_and_bad_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("crate.glb");
        fs::write(&path, fixture::clean_prop()).expect("write");
        let r = analyse(&path).expect("file");
        assert_eq!(r.bytes, fs::metadata(&path).expect("meta").len() as f64);
        fs::write(&path, b"nope").expect("write");
        assert_eq!(analyse(&path).expect_err("bad").code(), "invalid");
        assert!(analyse(&dir.path().join("missing.glb")).is_err());
    }

    #[test]
    fn base64_and_percent_decoding() {
        assert_eq!(base64_decode("TWFu").as_deref(), Some(&b"Man"[..]));
        assert_eq!(base64_decode("TWE=").as_deref(), Some(&b"Ma"[..]));
        assert_eq!(base64_decode("TQ==").as_deref(), Some(&b"M"[..]));
        assert!(base64_decode("T!==").is_none());
        assert_eq!(percent_decode("my%20model.bin"), "my model.bin");
    }
}
