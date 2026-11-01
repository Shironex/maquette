//! Camera stops for the Inspector's filmstrip: named viewpoints the camera
//! eases to. They come from the first of these that has any, in glTF
//! coordinates (Y up, metres):
//!
//! 1. `stops.json` in the project folder, written by hand:
//!    `{ "stops": [{ "name", "position": [x, y, z], "target": [x, y, z],
//!    "fov": 35 }] }` (a bare list works too; `fov` is the vertical field of
//!    view in degrees and defaults to 35; `poster` is an optional image
//!    path relative to the project).
//! 2. The stops of the scene layout (`layout.json`), placed in the app.
//! 3. `web/posters/stops.json`, written by the Capture posters step. Each
//!    entry names its camera and poster file (and its `label`, for a stop
//!    from the layout), and the starter script also writes the camera's
//!    position, target and field of view.
//! 4. Cameras inside the scene's models, by node name. A glTF camera has no
//!    target, so the stop looks along the camera's view axis, as far ahead
//!    as the scene's centre (at least a metre).
//!
//! A poster from `web/posters` is attached to any stop whose name matches
//! its camera (compared in snake case), so the filmstrip shows the rendered
//! still where there is one and a live render of the stop otherwise.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::inbox::math::{self, Mat4};

/// The vertical field of view a stop gets when its file names none.
pub const DEFAULT_FOV: f32 = 35.0;
/// Where the Capture posters step writes its stops, relative to the project.
pub const POSTERS_STOPS: &str = "web/posters/stops.json";
/// The hand written stops file, relative to the project.
pub const PROJECT_STOPS: &str = "stops.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraStopDef {
    pub name: String,
    pub position: [f32; 3],
    pub target: [f32; 3],
    /// Vertical field of view in degrees.
    pub fov: f32,
    /// An absolute path to the stop's poster render, when there is one.
    pub poster: Option<String>,
}

/// Where the stops came from, for the "From stops.json" line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum StopsSource {
    ProjectFile,
    Layout,
    Posters,
    ModelCameras,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CameraStops {
    pub source: StopsSource,
    /// The file the stops were read from, relative to the project.
    pub file: Option<String>,
    pub stops: Vec<CameraStopDef>,
}

#[derive(Deserialize)]
struct RawStop {
    #[serde(alias = "camera")]
    name: Option<String>,
    /// The stop's own name, when its camera name is made from it.
    label: Option<String>,
    position: Option<[f32; 3]>,
    target: Option<[f32; 3]>,
    fov: Option<f32>,
    #[serde(alias = "file")]
    poster: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawFile {
    List(Vec<RawStop>),
    Wrapped { stops: Vec<RawStop> },
}

fn read_raw(path: &Path) -> Option<Vec<RawStop>> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<RawFile>(&text) {
        Ok(RawFile::List(list) | RawFile::Wrapped { stops: list }) => Some(list),
        Err(err) => {
            tracing::warn!(file = %path.display(), error = %err, "unreadable camera stops");
            None
        }
    }
}

fn finite(v: [f32; 3]) -> bool {
    v.iter().all(|c| c.is_finite())
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    out
}

/// A label from a camera name: `stop_01_porch` reads `Porch`.
pub fn stop_label(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| c == '_' || c == '-' || c.is_whitespace())
        .filter(|w| !w.is_empty())
        .collect();
    let skip = usize::from(
        words.len() > 1
            && words
                .first()
                .is_some_and(|w| w.eq_ignore_ascii_case("stop")),
    );
    let mut rest: Vec<&str> = words[skip..].to_vec();
    if rest.len() > 1 && rest[0].chars().all(|c| c.is_ascii_digit()) {
        rest.remove(0);
    }
    let text = rest.join(" ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => name.to_string(),
    }
}

fn to_stops(raw: Vec<RawStop>, base: &Path) -> Vec<CameraStopDef> {
    raw.into_iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let (position, target) = (s.position?, s.target?);
            if !finite(position) || !finite(target) || position == target {
                return None;
            }
            let fov = s
                .fov
                .filter(|f| f.is_finite() && *f > 1.0 && *f < 170.0)
                .unwrap_or(DEFAULT_FOV);
            Some(CameraStopDef {
                name: s
                    .label
                    .filter(|l| !l.trim().is_empty())
                    .or_else(|| s.name.map(|n| stop_label(&n)))
                    .unwrap_or_else(|| format!("Stop {}", i + 1)),
                position,
                target,
                fov,
                poster: s
                    .poster
                    .map(|p| base.join(p))
                    .filter(|p| p.is_file())
                    .map(|p| p.display().to_string()),
            })
        })
        .collect()
}

/// Posters by the snake case of their stop label.
fn posters(project: &Path) -> Vec<(String, PathBuf)> {
    let dir = project.join("web/posters");
    read_raw(&project.join(POSTERS_STOPS))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| {
            let file = dir.join(s.poster?);
            let label = s
                .label
                .unwrap_or_else(|| stop_label(&s.name.unwrap_or_default()));
            file.is_file().then(|| (snake(&label), file))
        })
        .collect()
}

/// Camera nodes of one glTF file as stops looking at `center`'s depth.
pub fn cameras_in(file: &Path, center: [f32; 3]) -> Vec<CameraStopDef> {
    let Ok(bytes) = fs::read(file) else {
        return Vec::new();
    };
    let Ok(gltf) = gltf::Gltf::from_slice_without_validation(&bytes) else {
        return Vec::new();
    };
    let doc = &gltf.document;
    let worlds = crate::inbox::analyse::world_matrices(doc);
    doc.nodes()
        .filter_map(|node| {
            let camera = node.camera()?;
            let world: Mat4 = *worlds.get(node.index())?;
            let position = math::translation(&world);
            // A glTF camera looks down its local -Z.
            let ahead = math::point(&world, [0.0, 0.0, -1.0]);
            let mut forward = [
                ahead[0] - position[0],
                ahead[1] - position[1],
                ahead[2] - position[2],
            ];
            let len = math::distance(ahead, position);
            if len < 1e-6 {
                return None;
            }
            forward.iter_mut().for_each(|c| *c /= len);
            let to_center = [
                center[0] - position[0],
                center[1] - position[1],
                center[2] - position[2],
            ];
            let depth =
                (to_center[0] * forward[0] + to_center[1] * forward[1] + to_center[2] * forward[2])
                    .max(1.0);
            let fov = match camera.projection() {
                gltf::camera::Projection::Perspective(p) => p.yfov().to_degrees(),
                gltf::camera::Projection::Orthographic(_) => DEFAULT_FOV,
            };
            let name = node
                .name()
                .or(camera.name())
                .map_or_else(|| format!("Camera {}", node.index() + 1), stop_label);
            Some(CameraStopDef {
                name,
                position,
                target: [
                    position[0] + forward[0] * depth,
                    position[1] + forward[1] * depth,
                    position[2] + forward[2] * depth,
                ],
                fov,
                poster: None,
            })
        })
        .collect()
}

/// The stops of `project`, its scene models being `models` (with their
/// sizes, for the scene's centre).
pub fn load(project: &Path, models: &[(PathBuf, [f32; 3])]) -> CameraStops {
    let mut found = None;
    // Poster paths are relative to the file that names them.
    if let Some(raw) = read_raw(&project.join(PROJECT_STOPS)) {
        let stops = to_stops(raw, project);
        if !stops.is_empty() {
            found = Some(CameraStops {
                source: StopsSource::ProjectFile,
                file: Some(PROJECT_STOPS.to_string()),
                stops,
            });
        }
    }
    if found.is_none() {
        let layout = crate::layout::load(project).unwrap_or_default();
        if !layout.stops.is_empty() {
            found = Some(CameraStops {
                source: StopsSource::Layout,
                file: Some(crate::layout::LAYOUT_FILE.to_string()),
                stops: layout
                    .stops
                    .into_iter()
                    .map(|s| CameraStopDef {
                        name: s.name,
                        position: s.position,
                        target: s.target,
                        fov: s.fov,
                        poster: None,
                    })
                    .collect(),
            });
        }
    }
    if found.is_none()
        && let Some(raw) = read_raw(&project.join(POSTERS_STOPS))
    {
        let stops = to_stops(raw, &project.join("web/posters"));
        if !stops.is_empty() {
            found = Some(CameraStops {
                source: StopsSource::Posters,
                file: Some(POSTERS_STOPS.to_string()),
                stops,
            });
        }
    }
    let mut out = found.unwrap_or_else(|| {
        for (file, size) in models {
            // Models are exported around their own origin, so a camera in
            // one looks at that model's middle height.
            let center = [0.0, size[1] / 2.0, 0.0];
            let stops = cameras_in(file, center);
            if !stops.is_empty() {
                let name = file.file_name().map(|n| n.to_string_lossy().into_owned());
                return CameraStops {
                    source: StopsSource::ModelCameras,
                    file: name,
                    stops,
                };
            }
        }
        CameraStops {
            source: StopsSource::None,
            file: None,
            stops: Vec::new(),
        }
    });
    let posters = posters(project);
    for stop in out.stops.iter_mut().filter(|s| s.poster.is_none()) {
        let key = snake(&stop.name);
        if let Some((_, file)) = posters.iter().find(|(k, _)| *k == key) {
            stop.poster = Some(file.display().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write(path: &Path, value: &serde_json::Value) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, serde_json::to_vec(value).expect("json")).expect("write");
    }

    #[test]
    fn labels_read_like_names() {
        assert_eq!(stop_label("stop_01_porch"), "Porch");
        assert_eq!(stop_label("Stop Moon"), "Moon");
        assert_eq!(stop_label("pullback"), "Pullback");
        assert_eq!(stop_label("stop_02_shop_front"), "Shop front");
        assert_eq!(stop_label("stop"), "Stop");
    }

    #[test]
    fn the_project_file_comes_first_and_posters_attach_by_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project = dir.path();
        write(
            &project.join(POSTERS_STOPS),
            &json!({ "width": 1920, "height": 1080, "stops": [
                { "camera": "stop_01_porch", "file": "stop_01_porch.png",
                  "position": [9.0, 9.0, 9.0], "target": [0.0, 0.0, 0.0], "fov": 30 },
            ]}),
        );
        fs::write(project.join("web/posters/stop_01_porch.png"), b"png").expect("poster");
        // Only the posters file: its stops are used, poster included.
        let from_posters = load(project, &[]);
        assert_eq!(from_posters.source, StopsSource::Posters);
        assert_eq!(from_posters.stops[0].name, "Porch");
        assert!(
            from_posters.stops[0]
                .poster
                .as_deref()
                .is_some_and(|p| p.ends_with("stop_01_porch.png"))
        );

        write(
            &project.join(PROJECT_STOPS),
            &json!([
                { "name": "Porch", "position": [1.0, 2.0, 3.0], "target": [0.0, 1.0, 0.0] },
                { "name": "Broken", "position": [1.0, 2.0, 3.0] },
                { "name": "Moon", "position": [0.0, 5.0, 9.0], "target": [0.0, 4.0, 0.0], "fov": 50 },
            ]),
        );
        let stops = load(project, &[]);
        assert_eq!(stops.source, StopsSource::ProjectFile);
        assert_eq!(stops.file.as_deref(), Some("stops.json"));
        let names: Vec<&str> = stops.stops.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Porch", "Moon"]);
        assert!((stops.stops[0].fov - DEFAULT_FOV).abs() < f32::EPSILON);
        assert!(stops.stops[0].poster.is_some(), "the porch poster attaches");
        assert!(stops.stops[1].poster.is_none());
    }

    #[test]
    fn layout_stops_come_before_posters_and_find_theirs_by_label() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project = dir.path();
        write(
            &project.join(POSTERS_STOPS),
            &json!({ "stops": [
                { "camera": "stop_02_3d_model", "label": "3D model", "file": "stop_02_3d_model.png",
                  "position": [1.0, 1.0, 1.0], "target": [0.0, 0.0, 0.0], "fov": 30 },
            ]}),
        );
        fs::write(project.join("web/posters/stop_02_3d_model.png"), b"png").expect("poster");
        let from_posters = load(project, &[]);
        assert_eq!(from_posters.stops[0].name, "3D model", "the label wins");

        let layout = crate::layout::SceneLayout {
            stops: ["Concept", "3D model"]
                .iter()
                .enumerate()
                .map(|(i, name)| crate::layout::LayoutStop {
                    id: crate::project::slugify(name),
                    name: (*name).to_string(),
                    position: [0.0, 1.0, 3.0 + i as f32],
                    target: [0.0, 0.5, 0.0],
                    fov: 40.0,
                    models: None,
                })
                .collect(),
            ..Default::default()
        };
        crate::layout::update(project, |_| Ok(layout.clone())).expect("layout");
        let stops = load(project, &[]);
        assert_eq!(stops.source, StopsSource::Layout);
        assert_eq!(stops.file.as_deref(), Some("layout.json"));
        let names: Vec<&str> = stops.stops.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Concept", "3D model"]);
        assert!(stops.stops[0].poster.is_none());
        assert!(
            stops.stops[1]
                .poster
                .as_deref()
                .is_some_and(|p| p.ends_with("stop_02_3d_model.png"))
        );
    }

    #[test]
    fn cameras_in_a_model_become_stops() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("scene.gltf");
        // A camera five metres up the +Z axis looking back at the origin,
        // under a parent moved one metre up.
        write(
            &file,
            &json!({
                "asset": { "version": "2.0" },
                "scene": 0,
                "scenes": [{ "nodes": [0] }],
                "nodes": [
                    { "name": "Rig", "translation": [0.0, 1.0, 0.0], "children": [1] },
                    { "name": "stop_02_moon", "camera": 0, "translation": [0.0, 0.0, 5.0] },
                ],
                "cameras": [{ "type": "perspective",
                    "perspective": { "yfov": 0.5, "znear": 0.1 } }],
            }),
        );
        let stops = cameras_in(&file, [0.0, 1.0, 0.0]);
        assert_eq!(stops.len(), 1);
        let stop = &stops[0];
        assert_eq!(stop.name, "Moon");
        assert_eq!(stop.position, [0.0, 1.0, 5.0]);
        assert!((stop.target[2]).abs() < 1e-4, "{:?}", stop.target);
        assert!((stop.fov - 0.5f32.to_degrees()).abs() < 1e-3);
        let loaded = load(dir.path(), &[(file, [0.0, 2.0, 0.0])]);
        assert_eq!(loaded.source, StopsSource::ModelCameras);
        assert_eq!(loaded.file.as_deref(), Some("scene.gltf"));
    }
}
