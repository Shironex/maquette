//! Assembling a prop: the checks on a request, and (with a local Blender,
//! skipped without one) the node tree and pivots of a sigil gate made of
//! three models, landing as a version with its recipe kept.

use std::path::{Path, PathBuf};

use super::*;
use crate::assets::{NewAsset, Stage};
use crate::project::{NewProject, ProjectTemplate};
use crate::test_blender::{blender, glb_json, parts_model, toolbox};
use crate::workspace::{self, WorkspaceConfig};

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    config: WorkspaceConfig,
    project: String,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
        let manifest = project::create(
            &root,
            &config.layout,
            NewProject {
                name: "Frostveil".into(),
                template: ProjectTemplate::FloatingDiorama,
                target: Default::default(),
                game: None,
                site_repo: None,
                source_folder: None,
            },
        )
        .expect("project");
        Self {
            _dir: dir,
            root,
            config,
            project: manifest.id,
        }
    }

    fn dir(&self) -> PathBuf {
        project::dir(&self.root, &self.config.layout, &self.project).expect("dir")
    }

    /// An asset named `name`; with `model`, that file as its first version.
    fn asset(&self, name: &str, model: Option<&Path>) -> String {
        let models = self.dir().join("models");
        std::fs::create_dir_all(&models).expect("models");
        let mut id = String::new();
        project::modify(&self.root, &self.config.layout, &self.project, |m| {
            id = assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?
            .id;
            if let Some(model) = model {
                let file = format!("{}_v1.glb", id.replace('-', "_"));
                std::fs::copy(model, models.join(&file)).expect("copy");
                assets::add_version(m, &id, format!("models/{file}"), None)?;
            } else {
                assets::add_version(m, &id, "models/missing.glb".into(), None)?;
            }
            Ok(())
        })
        .expect("asset");
        id
    }

    fn manifest(&self) -> ProjectManifest {
        project::load(&self.root, &self.config.layout, &self.project).expect("load")
    }
}

fn part(asset: &str, name: &str, pivot: Pivot, at: Option<[f32; 3]>) -> AssemblePart {
    AssemblePart {
        asset: asset.into(),
        version: None,
        name: name.into(),
        pivot,
        at,
        turn: 0.0,
    }
}

#[test]
fn a_request_needs_two_named_parts_with_models_and_never_the_prop_itself() {
    let world = World::new();
    let frame = world.asset("Sigil gate frame", None);
    let door = world.asset("Sigil gate door", None);
    let gate = world.asset("Sigil gate", None);
    let manifest = world.manifest();
    let mut request = AssembleRequest {
        project: world.project.clone(),
        asset: None,
        name: "sigil_gate".into(),
        parts: vec![part(&frame, "frame", Pivot::BottomCentre, None)],
    };
    assert!(check(&manifest, &request).is_err(), "one part is no prop");
    request
        .parts
        .push(part(&door, "frame", Pivot::LeftHinge, None));
    let err = check(&manifest, &request).expect_err("same name twice");
    assert!(err.to_string().contains("Two parts"), "{err}");
    request.parts[1].name = "door_leaf".into();
    assert_eq!(
        check(&manifest, &request).expect("fine"),
        ["models/missing.glb", "models/missing.glb"]
    );
    request.parts[1].at = Some([0.0, f32::NAN, 0.0]);
    assert!(check(&manifest, &request).is_err(), "not a number");
    request.parts[1].at = Some([0.0, 2000.0, 0.0]);
    assert!(check(&manifest, &request).is_err(), "too far");
    request.parts[1].at = None;
    request.asset = Some(gate.clone());
    request.parts.push(part(&gate, "gate", Pivot::Origin, None));
    let err = check(&manifest, &request).expect_err("itself");
    assert!(err.to_string().contains("the prop itself"), "{err}");
    request.parts.pop();
    request.parts[0].version = Some(4);
    assert!(check(&manifest, &request).is_err(), "no v4");
}

#[test]
fn the_sigil_gate_assembles_into_one_named_node_tree_with_its_pivots() {
    let Some(blender) = blender() else { return };
    let world = World::new();
    let fixtures = tempfile::tempdir().expect("tempdir");
    let make = |which| parts_model(&blender, fixtures.path(), which);
    let frame = world.asset("Sigil gate frame", Some(&make("gate-frame")));
    let door = world.asset("Sigil gate door", Some(&make("gate-door")));
    let steps = world.asset("Sigil gate steps", Some(&make("gate-steps")));
    let mut steps_part = part(&steps, "steps", Pivot::BottomCentre, Some([0.0, 0.0, 0.4]));
    steps_part.turn = 90.0;
    let request = AssembleRequest {
        project: world.project.clone(),
        asset: None,
        name: "sigil_gate".into(),
        parts: vec![
            part(&frame, "frame", Pivot::BottomCentre, None),
            part(&door, "door_leaf", Pivot::LeftHinge, Some([-0.6, 0.0, 0.0])),
            steps_part,
        ],
    };
    let work = fixtures.path().join("work");
    let made = assemble(
        &world.root,
        &world.config.layout,
        &work,
        &request,
        &toolbox(&blender),
    )
    .expect("assemble");
    assert_eq!(made.root, "sigil_gate");
    assert_eq!(made.version.version, 1, "a new asset named after the prop");
    let asset = assets::find(&made.project, &made.asset).expect("asset");
    assert_eq!(asset.name, "sigil_gate");
    let names: Vec<&str> = made.nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["frame", "door_leaf", "steps"]);
    assert_eq!(made.nodes[1].at, [-0.6, 0.0, 0.0]);

    let doc = glb_json(&world.dir().join(&made.version.file));
    let nodes = doc["nodes"].as_array().expect("nodes");
    let by_name = |name: &str| {
        nodes
            .iter()
            .position(|n| n["name"] == name)
            .unwrap_or_else(|| panic!("no node {name}"))
    };
    let root = &nodes[by_name("sigil_gate")];
    let mut children: Vec<usize> = root["children"]
        .as_array()
        .expect("children")
        .iter()
        .filter_map(serde_json::Value::as_u64)
        .map(|c| c as usize)
        .collect();
    children.sort_unstable();
    let mut parts = vec![by_name("frame"), by_name("door_leaf"), by_name("steps")];
    parts.sort_unstable();
    assert_eq!(children, parts, "every part sits under the prop");
    // The door leaf turns on its left hinge: its mesh starts at x = 0.
    let door_node = &nodes[by_name("door_leaf")];
    let mesh = door_node["mesh"].as_u64().expect("mesh") as usize;
    let position = doc["meshes"][mesh]["primitives"][0]["attributes"]["POSITION"]
        .as_u64()
        .expect("position") as usize;
    let min_x = doc["accessors"][position]["min"][0].as_f64().expect("min");
    assert!(min_x.abs() < 1e-4, "hinge at the leaf's left edge: {min_x}");
    assert!(nodes[by_name("steps")].get("rotation").is_some(), "turned");

    let kept = recipe(&world.dir(), &made.asset).expect("recipe");
    assert_eq!(kept.parts.len(), 3);
    assert_eq!(kept.parts[1].pivot, Pivot::LeftHinge);
}
