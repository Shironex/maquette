use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::*;
use crate::assets::{AssetKind, NewAsset, Stage, add, add_version};
use crate::built::{self, BuiltSource};
use crate::layout::{self, ExplodeKey, ExplodeTrack, LayoutStop, Placement, ScenePath};
use crate::prepare::{self, ModelPrep};
use crate::project::{NewProject, ProjectTemplate};

/// Sends as a list, recording which ones a remove stopped.
#[derive(Default)]
struct Stub {
    jobs: Vec<Sent>,
    stopped: RefCell<Vec<String>>,
}

impl Sends for Stub {
    fn jobs(&self, _project: &str) -> Vec<Sent> {
        self.jobs.clone()
    }

    fn stop(&self, id: &str) -> CoreResult<()> {
        self.stopped.borrow_mut().push(id.to_string());
        self.jobs
            .iter()
            .find(|j| j.id == id)
            .map(|_| ())
            .ok_or_else(|| CoreError::NotFound(id.to_string()))
    }
}

fn job(id: &str, asset: &str, provider: &'static str, state: SentState, images: &[&str]) -> Sent {
    Sent {
        id: id.to_string(),
        asset: asset.to_string(),
        images: images.iter().map(|i| (*i).to_string()).collect(),
        provider,
        state,
    }
}

struct World {
    _dir: tempfile::TempDir,
    root: PathBuf,
    layout: FolderLayout,
    project: PathBuf,
}

impl World {
    fn ids(&self, ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    fn remove(&self, ids: &[&str], sends: &Stub) -> CoreResult<AssetsRemoved> {
        remove_assets(
            &self.root,
            &self.layout,
            "moon-island",
            &self.ids(ids),
            &Busy::default(),
            sends,
        )
    }

    fn manifest(&self) -> ProjectManifest {
        project::load(&self.root, &self.layout, "moon-island").expect("manifest")
    }

    fn trash(&self, id: &str) -> PathBuf {
        self.root.join(TRASH_DIR).join("moon-island").join(id)
    }
}

fn write(dir: &Path, file: &str, bytes: &str) {
    let path = dir.join(file);
    fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    fs::write(path, bytes).expect("write");
}

fn stop(id: &str, models: Option<Vec<&str>>) -> LayoutStop {
    LayoutStop {
        id: id.into(),
        name: id.into(),
        position: [1.0, 1.0, 1.0],
        target: [0.0, 0.0, 0.0],
        fov: 35.0,
        models: models.map(|m| m.into_iter().map(str::to_string).collect()),
    }
}

/// Moon Island with a clay bowl (two versions, a render, its refs, a
/// handoff folder, a built script, a placement, a stop, a path end, an
/// explode track and a real size) and a desk lamp beside it.
fn world() -> World {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let layout = FolderLayout::default();
    fs::create_dir_all(&root).expect("root");
    project::create(
        &root,
        &layout,
        NewProject {
            name: "Moon Island".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project = project::dir(&root, &layout, "moon-island").expect("dir");
    for (file, bytes) in [
        ("refs/clay_bowl.png", "front"),
        ("refs/clay_bowl_02.png", "side"),
        ("refs/shared_front.png", "shared"),
        ("refs/desk_lamp.png", "lamp"),
        ("models/clay_bowl_v1.glb", "glb 1"),
        ("models/clay_bowl_v2.glb", "glb 2"),
        ("thumbs/clay_bowl_v2.png", "render 2"),
        ("models/desk_lamp_v1.glb", "lamp glb"),
        ("handoff/clay-bowl-1/send.json", "{}"),
        ("built/clay_bowl.py", "def build(): pass"),
    ] {
        write(&project, file, bytes);
    }
    project::modify(&root, &layout, "moon-island", |m| {
        for name in ["Clay bowl", "Desk lamp"] {
            add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Blender,
                },
            )?;
        }
        add_version(m, "clay-bowl", "models/clay_bowl_v1.glb".into(), None)?;
        add_version(m, "clay-bowl", "models/clay_bowl_v2.glb".into(), None)?;
        add_version(m, "desk-lamp", "models/desk_lamp_v1.glb".into(), None)?;
        m.assets[0].thumbnail = Some("thumbs/clay_bowl_v2.png".into());
        Ok(())
    })
    .expect("assets");
    let mut scene = layout::SceneLayout::default();
    for id in ["clay-bowl", "desk-lamp"] {
        scene.models.insert(id.into(), Placement::default());
    }
    scene.stops = vec![
        stop("porch", Some(vec!["clay-bowl", "desk-lamp"])),
        stop("hall", None),
    ];
    scene.paths = vec![ScenePath {
        id: "glow".into(),
        name: "Glow".into(),
        curve: layout::PathCurve::Polyline,
        points: vec![[0.0; 3], [1.0, 0.0, 0.0]],
        start: Some("clay-bowl".into()),
        end: Some("desk-lamp".into()),
        width: 0.01,
        color: layout::paths::DEFAULT_COLOR.into(),
        glow: layout::paths::DEFAULT_GLOW.into(),
    }];
    scene.timeline.explode.insert(
        "clay-bowl".into(),
        ExplodeTrack {
            keys: vec![ExplodeKey {
                stop: "porch".into(),
                amount: 1.0,
            }],
            ..ExplodeTrack::default()
        },
    );
    layout::save(&project, &scene).expect("layout");
    let prep = ModelPrep {
        skip_bake: Some(true),
        ..ModelPrep::default()
    };
    prepare::save_entry(&project, "clay-bowl", &prep).expect("prep");
    built::update(&project, |file| {
        file.assets.insert(
            "clay-bowl".into(),
            BuiltSource {
                script: "built/clay_bowl.py".into(),
                helpers: Vec::new(),
                root: None,
                last: None,
            },
        );
        Ok(())
    })
    .expect("built");
    World {
        _dir: dir,
        root,
        layout,
        project,
    }
}

fn sends() -> Stub {
    Stub {
        jobs: vec![
            job(
                "clay-bowl-1",
                "clay-bowl",
                "By hand",
                SentState::Waiting,
                &["refs/shared_front.png"],
            ),
            job(
                "desk-lamp-1",
                "desk-lamp",
                "Tripo",
                SentState::Done,
                &["refs/shared_front.png"],
            ),
        ],
        stopped: RefCell::default(),
    }
}

#[test]
fn a_removed_asset_goes_to_the_trash_with_its_files_and_links() {
    let w = world();
    let sends = sends();
    let done = w.remove(&["clay-bowl"], &sends).expect("remove");
    let out = &done.outcomes[0];
    assert_eq!(out.state, RemoveState::Removed, "{out:?}");
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    assert_eq!(out.stopped, ["clay-bowl-1"]);
    assert_eq!(*sends.stopped.borrow(), ["clay-bowl-1"]);
    let id = out.trash.clone().expect("trash id");
    assert!(id.starts_with("clay-bowl-"));
    assert_eq!(
        out.trash_path.as_deref(),
        Some(format!("trash/moon-island/{id}").as_str())
    );

    // The Board and the files.
    assert_eq!(done.project.assets.len(), 1);
    assert_eq!(done.project.assets[0].id, "desk-lamp");
    let trash = w.trash(&id);
    for gone in [
        "models/clay_bowl_v1.glb",
        "models/clay_bowl_v2.glb",
        "thumbs/clay_bowl_v2.png",
        "refs/clay_bowl.png",
        "refs/clay_bowl_02.png",
        "handoff/clay-bowl-1",
        "built/clay_bowl.py",
    ] {
        assert!(!w.project.join(gone).exists(), "{gone} left the project");
        assert!(
            trash.join("files").join(gone).exists(),
            "{gone} is in the trash"
        );
    }
    // The lamp's own files and the ref its send used stay.
    for stays in [
        "refs/shared_front.png",
        "refs/desk_lamp.png",
        "models/desk_lamp_v1.glb",
    ] {
        assert!(w.project.join(stays).exists(), "{stays} stays");
    }
    assert_eq!(
        fs::read_to_string(trash.join("files/models/clay_bowl_v2.glb")).expect("v2"),
        "glb 2"
    );
    assert_eq!(out.files, 7);

    // The record.
    let record: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(trash.join("removed.json")).expect("record"))
            .expect("json");
    assert_eq!(record["asset"]["id"], "clay-bowl");
    assert_eq!(record["files"].as_array().expect("files").len(), 7);
    assert_eq!(record["links"]["stops"], json!(["porch"]));
    assert_eq!(record["links"]["pathStarts"], json!(["glow"]));
    assert_eq!(record["links"]["prep"]["skipBake"], true);
    assert_eq!(record["stopped"], json!(["clay-bowl-1"]));

    // Nothing points at it any more.
    let scene = layout::load(&w.project).expect("layout");
    assert!(!scene.models.contains_key("clay-bowl"));
    assert!(scene.models.contains_key("desk-lamp"));
    assert_eq!(
        scene.stops[0].models.as_deref(),
        Some(&["desk-lamp".to_string()][..])
    );
    assert_eq!(scene.paths[0].start, None);
    assert_eq!(scene.paths[0].end.as_deref(), Some("desk-lamp"));
    assert!(scene.timeline.explode.is_empty());
    assert!(prepare::load(&w.project).expect("prep").models.is_empty());
    assert!(built::load(&w.project).expect("built").assets.is_empty());

    let listed = list_trash(&w.root, &w.layout, "moon-island").expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].name, "Clay bowl");
    assert_eq!(listed[0].versions, 2);
}

#[test]
fn restore_puts_the_asset_its_files_and_links_back() {
    let w = world();
    let done = w.remove(&["clay-bowl"], &sends()).expect("remove");
    let id = done.outcomes[0].trash.clone().expect("trash id");
    let back = restore(&w.root, &w.layout, "moon-island", &id, &Busy::default()).expect("restore");
    assert!(back.warnings.is_empty(), "{:?}", back.warnings);
    assert_eq!(back.asset.id, "clay-bowl");
    assert_eq!(back.files, 7);
    assert_eq!(back.stopped, ["clay-bowl-1"]);
    let bowl = crate::assets::find(&back.project, "clay-bowl").expect("bowl");
    assert_eq!(bowl.versions.len(), 2);
    assert_eq!(bowl.thumbnail.as_deref(), Some("thumbs/clay_bowl_v2.png"));
    for file in [
        "models/clay_bowl_v1.glb",
        "thumbs/clay_bowl_v2.png",
        "refs/clay_bowl.png",
        "handoff/clay-bowl-1/send.json",
        "built/clay_bowl.py",
    ] {
        assert!(w.project.join(file).exists(), "{file} is back");
    }
    let scene = layout::load(&w.project).expect("layout");
    assert!(scene.models.contains_key("clay-bowl"));
    assert_eq!(
        scene.stops[0].models.as_deref(),
        Some(&["desk-lamp".to_string(), "clay-bowl".to_string()][..])
    );
    assert_eq!(scene.paths[0].start.as_deref(), Some("clay-bowl"));
    assert_eq!(scene.timeline.explode["clay-bowl"].keys.len(), 1);
    assert!(prepare::load(&w.project).expect("prep").models["clay-bowl"].skips_bake());
    assert!(
        built::load(&w.project)
            .expect("built")
            .assets
            .contains_key("clay-bowl")
    );
    assert!(!w.trash(&id).exists(), "the trash folder goes");
    assert!(
        list_trash(&w.root, &w.layout, "moon-island")
            .expect("list")
            .is_empty()
    );

    // A second restore of the same id has nothing to restore.
    let again = restore(&w.root, &w.layout, "moon-island", &id, &Busy::default());
    assert!(matches!(again, Err(CoreError::NotFound(_))), "{again:?}");
}

#[test]
fn restore_never_writes_over_what_came_back_meanwhile() {
    let w = world();
    let done = w.remove(&["clay-bowl"], &sends()).expect("remove");
    let id = done.outcomes[0].trash.clone().expect("trash id");
    write(&w.project, "refs/clay_bowl.png", "a new picture");
    let refused = restore(&w.root, &w.layout, "moon-island", &id, &Busy::default());
    assert!(matches!(refused, Err(CoreError::Invalid(ref m)) if m.contains("refs/clay_bowl.png")));
    assert_eq!(
        fs::read_to_string(w.project.join("refs/clay_bowl.png")).expect("ref"),
        "a new picture"
    );
    assert!(w.trash(&id).join("files/models/clay_bowl_v1.glb").exists());

    fs::remove_file(w.project.join("refs/clay_bowl.png")).expect("rm");
    project::modify(&w.root, &w.layout, "moon-island", |m| {
        add(
            m,
            NewAsset {
                name: "Clay bowl".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("again");
    let taken = restore(&w.root, &w.layout, "moon-island", &id, &Busy::default());
    assert!(matches!(taken, Err(CoreError::Invalid(ref m)) if m.contains("on the Board again")));
    let bad = restore(&w.root, &w.layout, "moon-island", "../x", &Busy::default());
    assert!(matches!(bad, Err(CoreError::NotFound(_))));
}

#[test]
fn a_running_pipeline_refuses_the_whole_remove_and_restore() {
    let w = world();
    let busy = Busy {
        run: Some("run-7"),
        building: &[],
    };
    let refused = remove_assets(
        &w.root,
        &w.layout,
        "moon-island",
        &w.ids(&["clay-bowl"]),
        &busy,
        &sends(),
    );
    assert!(
        matches!(refused, Err(CoreError::Invalid(ref m)) if m.contains("pipeline run (run-7) is going in Moon Island")),
        "{refused:?}"
    );
    assert_eq!(w.manifest().assets.len(), 2);
    assert!(w.project.join("models/clay_bowl_v1.glb").exists());
    assert!(!w.root.join(TRASH_DIR).exists());
    let restored = restore(&w.root, &w.layout, "moon-island", "anything", &busy);
    assert!(matches!(restored, Err(CoreError::Invalid(_))));
}

#[test]
fn a_running_paid_send_or_build_refuses_that_asset_and_keeps_its_files() {
    let w = world();
    let mut sends = sends();
    sends.jobs[0] = job("clay-bowl-2", "clay-bowl", "Meshy", SentState::Running, &[]);
    let done = w
        .remove(&["clay-bowl", "desk-lamp"], &sends)
        .expect("remove");
    let bowl = &done.outcomes[0];
    assert_eq!(bowl.state, RemoveState::Refused);
    let reason = bowl.reason.as_deref().expect("reason");
    assert!(reason.contains("Meshy is making the model of Clay bowl now (send clay-bowl-2)"));
    assert!(reason.contains("cancel it in the 3D queue"));
    assert!(sends.stopped.borrow().is_empty());
    assert!(w.project.join("models/clay_bowl_v1.glb").exists());
    // The other asset of the batch still goes.
    assert_eq!(done.outcomes[1].state, RemoveState::Removed);
    assert_eq!(done.project.assets.len(), 1);
    assert_eq!(done.project.assets[0].id, "clay-bowl");

    let building = ["clay-bowl".to_string()];
    let busy = Busy {
        run: None,
        building: &building,
    };
    let built = remove_assets(
        &w.root,
        &w.layout,
        "moon-island",
        &w.ids(&["clay-bowl"]),
        &busy,
        &Stub::default(),
    )
    .expect("remove");
    assert_eq!(built.outcomes[0].state, RemoveState::Refused);
    assert!(
        built.outcomes[0]
            .reason
            .as_deref()
            .is_some_and(|r| r.contains("building in Blender"))
    );
}

#[test]
fn removing_again_or_a_missing_asset_answers_not_found() {
    let w = world();
    let first = w
        .remove(&["clay-bowl", "nope", "clay-bowl"], &sends())
        .expect("remove");
    let states: Vec<RemoveState> = first.outcomes.iter().map(|o| o.state).collect();
    assert_eq!(states, [RemoveState::Removed, RemoveState::NotFound]);
    let again = w.remove(&["clay-bowl"], &sends()).expect("again");
    assert_eq!(again.outcomes[0].state, RemoveState::NotFound);
    assert_eq!(again.outcomes[0].trash, None);
    assert_eq!(again.project.assets.len(), 1);
    assert_eq!(
        list_trash(&w.root, &w.layout, "moon-island")
            .expect("list")
            .len(),
        1
    );
}

#[test]
fn a_remove_names_one_to_fifty_assets() {
    let w = world();
    assert!(matches!(
        w.remove(&[], &Stub::default()),
        Err(CoreError::Invalid(_))
    ));
    let many: Vec<String> = (0..=MAX_REMOVE).map(|n| format!("a-{n}")).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    assert!(matches!(
        w.remove(&refs, &Stub::default()),
        Err(CoreError::Invalid(ref m)) if m.contains("at most 50")
    ));
    assert!(matches!(
        remove_assets(
            &w.root,
            &w.layout,
            "no-such-project",
            &w.ids(&["clay-bowl"]),
            &Busy::default(),
            &Stub::default(),
        ),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn two_removes_of_assets_with_the_same_id_keep_both_in_the_trash() {
    let w = world();
    let first = w.remove(&["desk-lamp"], &Stub::default()).expect("first");
    project::modify(&w.root, &w.layout, "moon-island", |m| {
        add(
            m,
            NewAsset {
                name: "Desk lamp".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )
        .map(|_| ())
    })
    .expect("again");
    let second = w.remove(&["desk-lamp"], &Stub::default()).expect("second");
    assert_ne!(first.outcomes[0].trash, second.outcomes[0].trash);
    let listed = list_trash(&w.root, &w.layout, "moon-island").expect("list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, second.outcomes[0].trash.clone().expect("id"));
}
