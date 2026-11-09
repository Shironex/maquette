//! Joined clips and base edits in a real, headless Blender: the fixture
//! model takes the fixture clip as is, then joined with itself across a
//! blend, trimmed, retimed and frozen, and mirrored. Each bake's length
//! comes out as the timeline says, and a sampled bone holds the pose of the
//! clip frame the timeline names. Skipped where no Blender is installed.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use super::job::model_bone;
use super::tests::{Project, project_on};
use super::tests_blender::baked;
use super::*;
use crate::animations::list;
use crate::animations::tests_blender::{Setup, fixture, import_fixture, setup};
use crate::recipes::timeline::timeline;
use crate::recipes::{
    BlendWindow, LayerKind, LayerSettings, RecipeChange, RecipeFile, RecipeLayer, TimeEdit,
    TimeKind,
};
use crate::test_blender;
use crate::workspace::WorkspaceConfig;

/// A quaternion (x, y, z, w) or a vector, as glTF keeps them.
type Keys = (Vec<f64>, Vec<Vec<f64>>);

/// The staged model's JSON and binary chunk.
struct Glb {
    json: Value,
    bin: Vec<u8>,
}

impl Glb {
    fn read(file: &Path) -> Self {
        let bytes = std::fs::read(file).expect("glb");
        let len = u32::from_le_bytes(bytes[12..16].try_into().expect("len")) as usize;
        let json = test_blender::glb_json(file);
        let at = 20 + len;
        let bin_len = u32::from_le_bytes(bytes[at..at + 4].try_into().expect("bin")) as usize;
        Self {
            json,
            bin: bytes[at + 8..at + 8 + bin_len].to_vec(),
        }
    }

    fn accessor(&self, index: u64) -> Vec<Vec<f64>> {
        let a = &self.json["accessors"][index as usize];
        let view = &self.json["bufferViews"][a["bufferView"].as_u64().expect("view") as usize];
        let width = match a["type"].as_str() {
            Some("VEC4") => 4,
            Some("VEC3") => 3,
            _ => 1,
        };
        let stride = view["byteStride"].as_u64().unwrap_or(width * 4) as usize;
        let start = (view["byteOffset"].as_u64().unwrap_or(0)
            + a["byteOffset"].as_u64().unwrap_or(0)) as usize;
        (0..a["count"].as_u64().expect("count") as usize)
            .map(|i| {
                (0..width as usize)
                    .map(|k| {
                        let o = start + i * stride + k * 4;
                        f64::from(f32::from_le_bytes(
                            self.bin[o..o + 4].try_into().expect("f32"),
                        ))
                    })
                    .collect()
            })
            .collect()
    }

    fn node(&self, name: &str) -> usize {
        self.json["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .position(|n| n["name"] == name)
            .unwrap_or_else(|| panic!("no node {name}"))
    }

    /// The keys of `path` (rotation, translation) of node `node` in clip `clip`.
    fn keys(&self, clip: &str, node: usize, path: &str) -> Option<Keys> {
        let anim = self.json["animations"]
            .as_array()?
            .iter()
            .find(|a| a["name"] == clip)?;
        let channel = anim["channels"].as_array()?.iter().find(|c| {
            c["target"]["node"].as_u64() == Some(node as u64) && c["target"]["path"] == path
        })?;
        let sampler = &anim["samplers"][channel["sampler"].as_u64()? as usize];
        let times = self
            .accessor(sampler["input"].as_u64()?)
            .into_iter()
            .map(|v| v[0])
            .collect();
        Some((times, self.accessor(sampler["output"].as_u64()?)))
    }
}

/// `keys` at `t` seconds, linearly between keys (a quaternion renormalised).
fn sample((times, values): &Keys, t: f64) -> Vec<f64> {
    let i = times.iter().rposition(|x| *x <= t + 1e-6).unwrap_or(0);
    let j = (i + 1).min(times.len() - 1);
    let span = times[j] - times[i];
    let w = if span > 1e-9 {
        ((t - times[i]) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (a, mut b) = (values[i].clone(), values[j].clone());
    if a.len() == 4 && a.iter().zip(&b).map(|(x, y)| x * y).sum::<f64>() < 0.0 {
        b.iter_mut().for_each(|v| *v = -*v);
    }
    let mut out: Vec<f64> = a.iter().zip(&b).map(|(x, y)| x + (y - x) * w).collect();
    if out.len() == 4 {
        let n = out.iter().map(|v| v * v).sum::<f64>().sqrt();
        out.iter_mut().for_each(|v| *v /= n);
    }
    out
}

/// How far apart two turns are, degrees.
fn degrees_apart(a: &[f64], b: &[f64]) -> f64 {
    let dot = a
        .iter()
        .zip(b)
        .map(|(x, y)| x * y)
        .sum::<f64>()
        .abs()
        .min(1.0);
    2.0 * dot.acos().to_degrees()
}

type M = [[f64; 4]; 4];

fn mul(a: &M, b: &M) -> M {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| a[r][k] * b[k][c]).sum()))
}

fn trs(t: &[f64], q: &[f64], s: &[f64]) -> M {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    std::array::from_fn(|i| {
        if i == 3 {
            [0.0, 0.0, 0.0, 1.0]
        } else {
            [r[i][0] * s[0], r[i][1] * s[1], r[i][2] * s[2], t[i]]
        }
    })
}

/// Where node `node` is in the scene at `t` seconds of clip `clip`.
fn world(glb: &Glb, clip: &str, node: usize, t: f64) -> [f64; 3] {
    let nodes = glb.json["nodes"].as_array().expect("nodes");
    let mut parent = BTreeMap::new();
    for (i, n) in nodes.iter().enumerate() {
        for c in n["children"].as_array().into_iter().flatten() {
            parent.insert(c.as_u64().expect("child") as usize, i);
        }
    }
    let fixed = |n: &Value, key: &str, or: &[f64]| -> Vec<f64> {
        n[key].as_array().map_or_else(
            || or.to_vec(),
            |a| a.iter().filter_map(Value::as_f64).collect(),
        )
    };
    let local = |i: usize| {
        let n = &nodes[i];
        let t_ = glb
            .keys(clip, i, "translation")
            .map_or_else(|| fixed(n, "translation", &[0.0; 3]), |k| sample(&k, t));
        let q = glb.keys(clip, i, "rotation").map_or_else(
            || fixed(n, "rotation", &[0.0, 0.0, 0.0, 1.0]),
            |k| sample(&k, t),
        );
        trs(&t_, &q, &fixed(n, "scale", &[1.0; 3]))
    };
    let mut m = local(node);
    let mut at = node;
    while let Some(&p) = parent.get(&at) {
        m = mul(&local(p), &m);
        at = p;
    }
    [m[0][3], m[1][3], m[2][3]]
}

/// A project in `s` on the fixture clip, with a small nudge on the head (a
/// bake needs a change) and `change` made to its clips, time or mirror.
fn project(s: &Setup, change: impl FnOnce(&RecipeFile) -> RecipeChange) -> Project {
    let config = WorkspaceConfig::default();
    if list(s.root.path()).expect("clips").is_empty() {
        import_fixture(s, &config, "slide", "Foot Slide.fbx");
    }
    let clip = list(s.root.path()).expect("clips").remove(0).id;
    let file = s.files.join("model.glb");
    fixture(&s.blender, &s.root.path().join("scripts"), "model", &file);
    let mut p = project_on(
        s.root.path(),
        Some(&clip),
        std::fs::read(&file).expect("model"),
    );
    let nudge = RecipeLayer::new(
        "l-1",
        LayerKind::Nudge,
        "head",
        LayerSettings {
            rotation: Some([4.0, 0.0, 0.0]),
            ..LayerSettings::default()
        },
    );
    p.change(vec![nudge], Vec::new());
    let mut sent = change(&p.recipe.recipe);
    sent.expected = Some(p.recipe.version.clone());
    let manifest = crate::project::load(p.root(), &p.config.layout, &p.id).expect("manifest");
    p.recipe = crate::animations::recipes::update(p.root(), &p.dir, &manifest, &p.recipe.id, sent)
        .expect("change");
    p
}

/// Bakes `p` and answers the result, its staged model and the model bone
/// name of each VRM bone asked for.
fn bake_of(s: &Setup, p: &Project, vrm: &[&str]) -> (RecipeBakeResult, Glb, Vec<usize>) {
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let record = baked(s, p);
    assert_eq!(record.state, RecipeBakeState::Done, "{:?}", record.error);
    let result = record.result.expect("result");
    let glb = Glb::read(Path::new(&result.file));
    let nodes = vrm
        .iter()
        .map(|v| {
            let bone = model_bone(v, &prepared.map, &prepared.model.joints).expect("bone");
            glb.node(&bone)
        })
        .collect();
    (result, glb, nodes)
}

fn fps_of(glb: &Glb, result: &RecipeBakeResult) -> f64 {
    f64::from(result.frames) / test_blender::animation_seconds(&glb.json, &result.clip)
}

/// Checks that every timeline frame showing one whole clip frame (no blend
/// or hold in part) holds the reference bake's pose of `node` at that frame.
fn holds_the_named_frames(
    p: &Project,
    glb: &Glb,
    result: &RecipeBakeResult,
    plain: (&Glb, &RecipeBakeResult),
    node: usize,
) {
    let t = timeline(&p.recipe.recipe);
    assert_eq!(
        result.frames, t.frames,
        "the bake is as long as the timeline"
    );
    let fps = fps_of(glb, result);
    let plain_fps = fps_of(plain.0, plain.1);
    let ours = glb.keys(&result.clip, node, "rotation").expect("keys");
    let theirs = plain
        .0
        .keys(&plain.1.clip, node, "rotation")
        .expect("plain keys");
    let mut checked = 0;
    for (i, key) in t.keys.iter().enumerate() {
        let whole = key.to.is_none() && key.from.hold.as_ref().is_none_or(|h| h.weight >= 1.0);
        if !whole || (key.from.at - key.from.at.round()).abs() > 1e-6 {
            continue;
        }
        let a = sample(&ours, i as f64 / fps);
        let b = sample(&theirs, key.from.at / plain_fps);
        let apart = degrees_apart(&a, &b);
        assert!(
            apart < 1.0,
            "frame {i} (clip frame {}): {apart} degrees off",
            key.from.at
        );
        checked += 1;
    }
    assert!(checked > 5, "only {checked} frames checked");
}

#[test]
fn joined_trimmed_retimed_frozen_and_mirrored_bakes_play_the_timeline() {
    let Some(s) = setup() else { return };
    let arm = ["rightLowerArm", "rightFoot", "leftFoot"];
    let plain = project(&s, |_| RecipeChange::default());
    let (base, base_glb, nodes) = bake_of(&s, &plain, &arm);
    let frames = base.frames;
    assert!(frames >= 30, "the fixture clip runs {frames} frames");

    // Joined with itself across a blend.
    let joined = project(&s, |r| {
        let mut first = r.base.clone();
        first.trim = Some([0, 20]);
        let mut second = r.base.clone();
        second.trim = Some([5, 25]);
        second.blend = Some(BlendWindow {
            frames: 4,
            ..BlendWindow::default()
        });
        RecipeChange {
            clips: Some(vec![first, second]),
            kept: Some(vec![Some(0), Some(0)]),
            ..RecipeChange::default()
        }
    });
    let (result, glb, _) = bake_of(&s, &joined, &arm);
    assert_eq!(result.frames, 36, "20 and 20 kept, less a 4 frame blend");
    holds_the_named_frames(&joined, &glb, &result, (&base_glb, &base), nodes[0]);

    // Trimmed, a range played at twice the speed, and a freeze.
    let edited = project(&s, |r| {
        let mut clip = r.base.clone();
        clip.trim = Some([2, 28]);
        RecipeChange {
            clips: Some(vec![clip]),
            kept: Some(vec![Some(0)]),
            time: Some(vec![
                TimeEdit::retime(0, 8, 18, 2.0),
                TimeEdit::hold(TimeKind::Freeze, 0, 22, 6),
            ]),
            ..RecipeChange::default()
        }
    });
    let (result, glb, _) = bake_of(&s, &edited, &arm);
    assert_eq!(
        result.frames, 27,
        "26 kept, 5 saved by the retime, 6 frozen"
    );
    holds_the_named_frames(&edited, &glb, &result, (&base_glb, &base), nodes[0]);

    // Mirrored: the left foot goes where the right foot went, across the
    // model's middle (it faces +Z, so its left is +X).
    let mirrored = project(&s, |_| RecipeChange {
        mirror: Some(true),
        ..RecipeChange::default()
    });
    let (result, glb, _) = bake_of(&s, &mirrored, &arm);
    assert_eq!(result.frames, frames, "mirror keeps the length");
    let fps = fps_of(&glb, &result);
    let reflected = |a: [f64; 3], b: [f64; 3]| {
        ((a[0] + b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    let mut lopsided: f64 = 0.0;
    for f in 0..=frames {
        let t = f64::from(f) / fps;
        let right = world(&base_glb, &base.clip, nodes[1], t);
        let left = world(&base_glb, &base.clip, nodes[2], t);
        lopsided = lopsided.max(reflected(left, right));
    }
    eprintln!("the clip's feet are {lopsided} m from mirroring each other at most");
    assert!(
        lopsided > 0.03,
        "the clip is too even to tell a mirror ({lopsided} m)"
    );
    for f in [0, frames / 3, frames * 2 / 3, frames] {
        let t = f64::from(f) / fps;
        let right = world(&base_glb, &base.clip, nodes[1], t);
        let left = world(&glb, &result.clip, nodes[2], t);
        let gap = ((left[0] + right[0]).powi(2)
            + (left[1] - right[1]).powi(2)
            + (left[2] - right[2]).powi(2))
        .sqrt();
        assert!(
            gap < 0.015,
            "frame {f}: the left foot is {gap} m from the mirrored right"
        );
    }
}
