//! A replace on a real project folder in a temporary workspace: a scan and a
//! built asset with fixture models, no Blender. The built asset is made
//! current by hand (its model file, its version and a build record with the
//! key of its files), which is what a good build leaves.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::assets::remove::Sent;
use crate::assets::{AssetKind, AssetStatus, NewAsset, Stage, stats_of};
use crate::built::{self, BuildRecord, NewBuilt};
use crate::inbox::fixture;
use crate::layout::{self, LayoutStop, Placement};
use crate::project::{NewProject, ProjectTemplate};
use crate::workspace;

mod links;
mod refusals;
mod settings;
mod ship;
mod undo;
mod undone;

/// Sends as a list; nothing is ever stopped here.
#[derive(Default)]
pub(super) struct Stub(pub Vec<Sent>);

impl Sends for Stub {
    fn jobs(&self, _project: &str) -> Vec<Sent> {
        self.0.clone()
    }

    fn stop(&self, _project: &str, _id: &str) -> CoreResult<()> {
        Ok(())
    }
}

pub(super) struct World {
    _dir: tempfile::TempDir,
    pub root: PathBuf,
    pub config: WorkspaceConfig,
    pub project: String,
    pub dir: PathBuf,
    /// Where the scripts come from, like a folder of hand made pieces.
    pub pieces: PathBuf,
    pub sends: Stub,
}

/// The scan: a half metre crate with one node and one material, both
/// called "Crate".
pub(super) fn scan_model() -> Vec<u8> {
    fixture::clean_prop()
}

/// The built model: three boxes in a row, 30 cm long, each its own part
/// (`Part_0` to `Part_2`), one material called "Grey".
pub(super) fn built_model() -> Vec<u8> {
    fixture::split_parts(3)
}

impl World {
    pub(super) fn new(target: NewProject) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ws");
        let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
        let manifest = project::create(&root, &config.layout, target).expect("project");
        let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
        let pieces = dir.path().join("pieces");
        fs::create_dir_all(&pieces).expect("pieces");
        fs::write(pieces.join("common.py"), "PALETTE = 1\n").expect("common");
        Self {
            _dir: dir,
            root,
            config,
            project: manifest.id,
            dir: project_dir,
            pieces,
            sends: Stub::default(),
        }
    }

    /// A website project called Study room.
    pub(super) fn website() -> Self {
        Self::new(NewProject {
            name: "Study room".into(),
            template: ProjectTemplate::FloatingDiorama,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        })
    }

    pub(super) fn env(&self) -> ReplaceEnv<'_> {
        ReplaceEnv {
            root: &self.root,
            config: &self.config,
            project: &self.project,
            busy: Busy::default(),
            sends: &self.sends,
        }
    }

    pub(super) fn manifest(&self) -> ProjectManifest {
        project::load(&self.root, &self.config.layout, &self.project).expect("manifest")
    }

    pub(super) fn asset(&self, id: &str) -> Asset {
        assets::find(&self.manifest(), id).expect("asset").clone()
    }

    /// Writes `bytes` as the asset's next version, the way an accept or a
    /// build does, and returns the file.
    pub(super) fn add_model(&self, id: &str, bytes: &[u8]) -> String {
        let number = self.asset(id).next_version();
        let file = format!("models/{}_v{number}.glb", key(id));
        let path = self.dir.join(&file);
        fs::create_dir_all(path.parent().expect("models")).expect("models dir");
        fs::write(&path, bytes).expect("model");
        let stats = stats_of(&crate::inbox::analyse(&path).expect("analyse"));
        project::modify(&self.root, &self.config.layout, &self.project, |m| {
            assets::add_version(m, id, file.clone(), Some(stats))?;
            assets::set_status(m, id, AssetStatus::Done)
        })
        .expect("version");
        file
    }

    /// A provider model: an asset at Web with one accepted scan.
    pub(super) fn scan(&self, name: &str) -> Asset {
        let mut id = String::new();
        project::modify(&self.root, &self.config.layout, &self.project, |m| {
            id = assets::add(
                m,
                NewAsset {
                    name: name.into(),
                    kind: AssetKind::Prop,
                    stage: Stage::Web,
                },
            )?
            .id;
            Ok(())
        })
        .expect("asset");
        self.add_model(&id, &scan_model());
        self.asset(&id)
    }

    /// A built asset whose newest model is what its script makes.
    pub(super) fn built(&self, name: &str, script: &str) -> Asset {
        fs::write(
            self.pieces.join(script),
            "import common\ndef build():\n    pass\n",
        )
        .expect("script");
        let (asset, _) = built::add(
            &self.root,
            &self.config.layout,
            &self.project,
            &NewBuilt {
                name: name.into(),
                kind: AssetKind::Prop,
                script: self.pieces.join(script).display().to_string(),
                helpers: vec![self.pieces.join("common.py").display().to_string()],
                root: None,
                replace: false,
            },
        )
        .expect("add built");
        self.add_model(&asset.id, &built_model());
        self.mark_built(&asset.id);
        self.asset(&asset.id)
    }

    /// Records a good build of the asset's files as they are now.
    pub(super) fn mark_built(&self, id: &str) {
        let source = load(&self.dir).expect("built").assets[id].clone();
        let key = built::key(&self.dir, &source).expect("key");
        let version = self.asset(id).latest().map(|v| v.version);
        built::update(&self.dir, |file| {
            let entry = file.assets.get_mut(id).expect("entry");
            entry.last = Some(BuildRecord {
                key,
                at: now_rfc3339(),
                version,
                error: None,
                objects: vec!["Part_0".into(), "Part_1".into(), "Part_2".into()],
                materials: vec!["Grey".into()],
                triangles: 36,
                warnings: Vec::new(),
            });
            Ok(())
        })
        .expect("record");
    }

    /// Places the asset and lists it in a camera stop, as a scene does.
    pub(super) fn place(&self, id: &str) {
        layout::update(&self.dir, |current| {
            let mut next = current.clone();
            next.models.insert(
                id.to_string(),
                Placement {
                    position: [0.4, 0.0, -0.1],
                    rotation: [0.0, 35.0, 0.0],
                    scale: 1.0,
                },
            );
            next.stops.push(LayoutStop {
                id: "desk".into(),
                name: "Desk".into(),
                position: [0.9, 0.5, 1.4],
                target: [0.0, 0.15, 0.0],
                fov: 35.0,
                models: Some(vec![id.to_string()]),
            });
            Ok(next)
        })
        .expect("layout");
    }

    pub(super) fn replace(&self, asset: &str, built: &str) -> ReplaceDone {
        replace(&self.env(), asset, built).expect("replace")
    }

    pub(super) fn read(&self, file: &str) -> String {
        fs::read_to_string(self.dir.join(file)).unwrap_or_default()
    }
}

/// Every file under `dir` with its bytes, by its path relative to `dir`.
pub(super) fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("inside")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read(&path).expect("read"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn the_built_model_takes_the_place_and_the_asset_keeps_its_id() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.place(&planner.id);
    let layout_before = w.read("layout.json");
    let built_bytes = fs::read(w.dir.join("models/planner_built_v1.glb")).expect("built");

    let done = w.replace(&planner.id, &desk.id);

    // The asset keeps its id and its history, and the newest version is the built model.
    let after = w.asset("spiral-planner");
    let files: Vec<&str> = after.versions.iter().map(|v| v.file.as_str()).collect();
    assert_eq!(
        files,
        [
            "models/spiral_planner_v1.glb",
            "models/spiral_planner_v2.glb"
        ]
    );
    assert_eq!(
        fs::read(w.dir.join("models/spiral_planner_v2.glb")).expect("new"),
        built_bytes
    );
    assert_eq!(after.status, AssetStatus::Done);
    // The built asset's own card left the Board, into the asset trash.
    assert!(w.manifest().assets.iter().all(|a| a.id != desk.id));
    let trash = done.report.trash.clone().expect("trash id");
    let listed = remove::list_trash(&w.root, &w.config.layout, &w.project).expect("trash");
    assert_eq!(listed[0].id, trash);
    // It is a built asset now, current against the script, with what it was kept.
    let listed = built::list(&w.dir, &w.manifest(), &[]).expect("built list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].asset, "spiral-planner");
    assert_eq!(listed[0].state, BuiltState::Current);
    assert_eq!(listed[0].source.script, "built/planner.py");
    assert!(w.dir.join("built/planner.py").is_file(), "the script stays");
    let record = listed[0].source.replaced.clone().expect("record");
    assert_eq!(record.versions, [1]);
    assert_eq!(record.by.asset, "planner-built");
    assert_eq!(record.by.trash.as_deref(), Some(trash.as_str()));
    assert_eq!(
        listed[0].source.last.as_ref().and_then(|l| l.version),
        Some(2)
    );
    // The layout never changed: same id, same placement, same stop.
    assert_eq!(w.read("layout.json"), layout_before);
    // The report names both files with their numbers.
    let report = &done.report;
    assert_eq!(report.old.file, "models/spiral_planner_v1.glb");
    assert_eq!(report.new.file, "models/spiral_planner_v2.glb");
    assert_eq!(
        report.old.ships_as.as_deref(),
        Some("spiral_planner_v1.glb")
    );
    assert_eq!(
        report.new.ships_as.as_deref(),
        Some("spiral_planner_v2.glb")
    );
    assert_eq!(
        (report.old.triangles, report.new.triangles),
        (Some(12), Some(36))
    );
    assert!(report.old.bytes.is_some_and(|b| b > 0.0));
    assert!(report.new.bytes.is_some_and(|b| b > 0.0));
    assert_eq!(report.parts, ["Part_0", "Part_1", "Part_2"]);
    assert_eq!(report.note, NOTE);
}

#[test]
fn the_plan_reads_only() {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    let before = files(&w.dir);
    let report = plan(&w.env(), &planner.id, &desk.id).expect("plan");
    assert_eq!(report.new.version, 2);
    assert_eq!(report.trash, None);
    assert_eq!(files(&w.dir), before, "a plan writes nothing");
}
