//! Writes VRM 1.0 onto a GLB: the `VRMC_vrm` extension (humanoid, meta, a
//! default look at, first person left to auto), with the animations taken
//! out, node scale float noise snapped to 1 and, in MToon mode, the soft
//! toon shader on every material.
//!
//! Writing only adds: the JSON gains the extensions and, with a thumbnail,
//! one image and buffer view, and the BIN chunk gains the image bytes at
//! its end. Nodes, meshes and accessors keep their indices and bytes. The
//! animations' keyframes stay in the BIN chunk, unused.
//!
//! The writer refuses a model whose rig VRM cannot accept: no skin, a
//! required bone missing, one out of the humanoid order or skipping its
//! parent, swapped sides, a mirrored or uneven scale. What the rest pose
//! still lacks (the facing, the T-pose, the ground) is written as it is and
//! comes back as `unresolved`, for the caller to show; normalising the rest
//! pose is a step before this one.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::container::{self, invalid, set_extension, use_extension};
use super::meta::VrmMeta;
use super::pose::SCALE_NOISE;
use super::{BoneOverrides, FindingLevel, VrmFinding, assess, is_human_bone, mtoon, with_parsed};
use maquette_base::error::CoreResult;

/// The VRM extension.
pub const VRMC_VRM: &str = "VRMC_vrm";

/// How the materials are written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum MaterialMode {
    /// The model's PBR materials as they are.
    Pbr,
    /// MToon, soft: shaded from the base color, no outline.
    #[default]
    MtoonSoft,
}

/// A thumbnail for the meta: square, PNG or JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub bytes: Vec<u8>,
    /// `image/png` or `image/jpeg`.
    pub mime: String,
}

/// What to write.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WriteSpec {
    pub meta: VrmMeta,
    pub overrides: BoneOverrides,
    pub material: MaterialMode,
    pub thumbnail: Option<Thumbnail>,
}

/// A written VRM and what its rest pose still lacks.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    pub bytes: Vec<u8>,
    /// The blockers and warnings the file still carries, blockers first.
    pub unresolved: Vec<VrmFinding>,
}

/// The findings the writer refuses on. The other blockers are about the
/// rest pose, which a VRM runtime still loads.
const REFUSED: [&str; 7] = [
    "skin",
    "required-bones",
    "bone-gaps",
    "parents",
    "sides",
    "negative-scale",
    "uneven-scale",
];

/// The look at: the eye bones when the rig has both, otherwise
/// expressions, which this file does not define yet, so nothing moves.
fn look_at(has_eyes: bool) -> Value {
    let (kind, scale) = if has_eyes {
        ("bone", 10.0)
    } else {
        ("expression", 1.0)
    };
    let range = json!({ "inputMaxValue": 90.0, "outputScale": scale });
    json!({
        "offsetFromHeadBone": [0.0, 0.06, 0.0],
        "type": kind,
        "rangeMapHorizontalInner": range,
        "rangeMapHorizontalOuter": range,
        "rangeMapVerticalDown": range,
        "rangeMapVerticalUp": range,
    })
}

fn check_spec(spec: &WriteSpec) -> CoreResult<()> {
    spec.meta.validate()?;
    if let Some(bone) = spec.overrides.keys().find(|k| !is_human_bone(k)) {
        return Err(invalid(format!(
            "{bone} is not a VRM humanoid bone. Use the VRM 1.0 names, such as leftUpperArm."
        )));
    }
    if let Some(thumb) = &spec.thumbnail
        && thumb.mime != "image/png"
        && thumb.mime != "image/jpeg"
    {
        return Err(invalid("A VRM thumbnail is a PNG or JPEG image."));
    }
    Ok(())
}

fn sentence(f: &VrmFinding) -> String {
    match &f.detail {
        Some(detail) => format!("{}. {detail}", f.title),
        None => format!("{}.", f.title),
    }
}

/// Writes VRM 1.0 onto `glb` as `spec` says; `base` resolves outside
/// buffers while the rig is read.
///
/// # Errors
///
/// Refuses a model that is not a GLB or a rig VRM cannot accept, a meta
/// without a name or an author, an override on a name that is not a VRM
/// bone and a thumbnail that is not PNG or JPEG, each in sentences for the
/// person or agent.
pub fn write(glb: &[u8], spec: &WriteSpec, base: &Path) -> CoreResult<Written> {
    check_spec(spec)?;
    let (mut json, mut bin) = container::split(glb)?;
    let (readiness, mapped, meshes) = with_parsed(glb, base, |doc, buffers| {
        let (readiness, mapped) = assess(doc, buffers, &spec.overrides);
        let meshes: Vec<usize> = doc
            .nodes()
            .filter(|n| n.mesh().is_some())
            .map(|n| n.index())
            .collect();
        Ok((readiness, mapped, meshes))
    })?;
    let refused: Vec<String> = readiness
        .findings
        .iter()
        .filter(|f| f.level == FindingLevel::Blocker && REFUSED.contains(&f.id.as_str()))
        .map(sentence)
        .collect();
    if !refused.is_empty() {
        return Err(invalid(format!(
            "This model cannot be written as VRM yet. {}",
            refused.join(" ")
        )));
    }

    if let Some(root) = json.as_object_mut() {
        root.remove("animations");
    }
    container::snap_scales(&mut json, f64::from(SCALE_NOISE));
    if spec.material == MaterialMode::MtoonSoft {
        mtoon::apply_soft(&mut json);
    }
    let mut meta = serde_json::to_value(&spec.meta).map_err(|e| invalid(e.to_string()))?;
    if let Some(thumb) = &spec.thumbnail {
        let view = container::append_view(&mut json, &mut bin, &thumb.bytes)?;
        let image = container::push(
            &mut json,
            "images",
            json!({ "bufferView": view, "mimeType": thumb.mime, "name": "Thumbnail" }),
        );
        meta["thumbnailImage"] = json!(image);
    }

    let human_bones: Map<String, Value> = readiness
        .bones
        .iter()
        .filter_map(|b| {
            let (_, node) = mapped.iter().find(|(bone, _)| *bone == b.bone)?;
            Some((b.bone.clone(), json!({ "node": node })))
        })
        .collect();
    let has_eyes = human_bones.contains_key("leftEye") && human_bones.contains_key("rightEye");
    let annotations: Vec<Value> = meshes
        .iter()
        .map(|node| json!({ "node": node, "type": "auto" }))
        .collect();
    let vrm = json!({
        "specVersion": "1.0",
        "meta": meta,
        "humanoid": { "humanBones": human_bones },
        "firstPerson": { "meshAnnotations": annotations },
        "lookAt": look_at(has_eyes),
    });
    set_extension(&mut json, VRMC_VRM, vrm);
    use_extension(&mut json, VRMC_VRM);

    let unresolved = readiness
        .findings
        .into_iter()
        .filter(|f| f.level != FindingLevel::Note)
        .collect();
    Ok(Written {
        bytes: container::join(&json, bin)?,
        unresolved,
    })
}
