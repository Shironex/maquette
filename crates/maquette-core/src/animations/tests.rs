use std::fs::{self, File};
use std::io::Write as _;
use std::path::Path;

use super::blender::{ClipReport, FileReport, ReadReport};
use super::import::{self, ClipStatus, ImportRequest, PackChoice, StagedEntry, StagedFile};
use super::*;
use crate::workspace::WorkspaceConfig;

pub(crate) const MIXAMO: [&str; 22] = [
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
    "LeftToeBase",
    "RightUpLeg",
    "RightLeg",
    "RightFoot",
    "RightToeBase",
];

pub(crate) fn mixamo_bones() -> Vec<String> {
    MIXAMO.iter().map(|b| format!("mixamorig:{b}")).collect()
}

fn rest(names: &[String]) -> Vec<RestBone> {
    names
        .iter()
        .map(|n| RestBone {
            name: n.clone(),
            parent: None,
            head: [0.0, 1.0, 0.0],
            tail: [0.0, 1.1, 0.0],
        })
        .collect()
}

fn clip_report(hash: &str, keys: bool) -> ClipReport {
    ClipReport {
        name: "mixamo.com".into(),
        hash: hash.into(),
        has_keys: keys,
        frames: 38,
        fps: 30.0,
        length: 1.27,
        root_motion: Some(RootMotion {
            metres: 0.92,
            direction: "left".into(),
        }),
        loops: false,
        bytes: keys.then_some(46_000),
        out: keys.then(|| format!("{hash}/0.glb")),
    }
}

fn file_report(id: &str, bones: &[String], clips: Vec<ClipReport>) -> FileReport {
    FileReport {
        id: id.into(),
        error: None,
        skinned: true,
        bones: rest(bones),
        hips_height: 1.04,
        clips,
    }
}

fn single(index: usize, name: &str) -> StagedFile {
    StagedFile {
        name: name.into(),
        archive: false,
        bytes: 1_200_000,
        entries: vec![StagedEntry {
            id: format!("{index}-0"),
            file: name.into(),
            path: format!("in/{index}/0.fbx"),
        }],
        left_out: Vec::new(),
    }
}

fn pack(index: usize, name: &str, inner: &[&str]) -> StagedFile {
    StagedFile {
        name: name.into(),
        archive: true,
        bytes: 96_000_000,
        entries: inner
            .iter()
            .enumerate()
            .map(|(n, f)| StagedEntry {
                id: format!("{index}-{n}"),
                file: (*f).into(),
                path: format!("in/{index}/{n}.fbx"),
            })
            .collect(),
        left_out: Vec::new(),
    }
}

pub(crate) fn library_clip(id: &str, name: &str, pack: &str, hash: &str) -> AnimationClip {
    AnimationClip {
        version: CLIP_VERSION,
        id: id.into(),
        name: name.into(),
        pack: pack.into(),
        tags: names::tags_for(name),
        rig: ClipRig {
            kind: SourceRig::Mixamo,
            bones: rest(&mixamo_bones()),
            hips_height: 1.04,
        },
        frames: 38,
        fps: 30.0,
        length: 1.27,
        root_motion: None,
        loops: false,
        bytes: 46_000.0,
        file: CLIP_GLB.into(),
        origin: ClipOrigin {
            source: ClipSource::Import,
            file: format!("{name}.fbx"),
            archive: None,
            kept: None,
            project: None,
            asset: None,
            version: None,
        },
        hash: hash.into(),
        revision: 1,
        created_at: "2026-10-26T10:00:00.000Z".into(),
        updated_at: "2026-10-26T10:00:00.000Z".into(),
    }
}

fn keep(root: &Path, id: &str, name: &str, pack: &str, hash: &str) {
    store(root, &library_clip(id, name, pack, hash)).expect("store");
}

#[test]
fn clips_round_trip_and_list_by_pack_then_name() {
    let root = tempfile::tempdir().expect("tempdir");
    let sword = "Pro Sword and Shield Pack";
    keep(root.path(), "slash", "Sword And Shield Slash", sword, "a");
    keep(root.path(), "dodge", "Standing Dodge Left", SINGLES, "b");
    keep(root.path(), "block", "Sword And Shield Block", sword, "c");
    fs::create_dir_all(dir(root.path()).join("stray")).expect("stray");
    let names: Vec<String> = list(root.path())
        .expect("list")
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(
        names,
        [
            "Sword And Shield Block",
            "Sword And Shield Slash",
            "Standing Dodge Left"
        ]
    );
    assert_eq!(load(root.path(), "dodge").expect("load").pack, SINGLES);
    assert!(
        root.path()
            .join("library")
            .join("animations")
            .join("dodge")
            .join("clip.json")
            .is_file()
    );
    let shown = view(root.path(), &crate::workspace::FolderLayout::default()).expect("view");
    assert_eq!(shown.items.len(), 3);
    assert!(shown.disk_bytes > 0.0);
}

#[test]
fn clip_ids_and_files_stay_inside_the_library() {
    let root = tempfile::tempdir().expect("tempdir");
    assert!(clip_dir(root.path(), "../projects").is_err());
    assert!(clip_dir(root.path(), ".hidden").is_err());
    let mut clip = library_clip("bad", "Bad", SINGLES, "x");
    clip.file = "../../escape.glb".into();
    assert!(store(root.path(), &clip).is_err());
    assert!(matches!(
        load(root.path(), "missing"),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn a_clip_from_a_newer_maquette_is_refused() {
    let root = tempfile::tempdir().expect("tempdir");
    let mut clip = library_clip("later", "Later", SINGLES, "x");
    clip.version = CLIP_VERSION + 1;
    let folder = clip_dir(root.path(), "later").expect("dir");
    fs::create_dir_all(&folder).expect("dir");
    let text = serde_json::to_string(&clip).expect("json");
    fs::write(folder.join(CLIP_FILE), text).expect("write");
    assert!(matches!(
        load(root.path(), "later"),
        Err(CoreError::UnsupportedVersion { .. })
    ));
    assert!(list(root.path()).expect("list").is_empty());
}

#[test]
fn rigs_are_told_apart_by_their_bone_names() {
    assert_eq!(rig::detect(&mixamo_bones(), false), SourceRig::Mixamo);
    let bare: Vec<String> = MIXAMO.iter().map(|b| (*b).to_string()).collect();
    assert_eq!(rig::detect(&bare, false), SourceRig::Mixamo);
    let tripo: Vec<String> = rig::TRIPO_BONES
        .iter()
        .flat_map(|b| [b.replace("{s}", "L"), b.replace("{s}", "R")])
        .collect();
    assert_eq!(rig::detect(&tripo, false), SourceRig::Tripo);
    let custom: Vec<String> = ["root", "skirt_01", "hair_01", "spine_a"]
        .map(String::from)
        .to_vec();
    assert_eq!(rig::detect(&custom, false), SourceRig::Custom);
    assert_eq!(rig::detect(&custom, true), SourceRig::Tripo);
    assert_eq!(rig::prefix(&mixamo_bones()).as_deref(), Some("mixamorig:"));
    assert_eq!(rig::prefix(&custom), None);
}

#[test]
fn tags_and_names_come_from_the_clip_names() {
    assert_eq!(names::tags_for("Sword And Shield Slash"), ["combat"]);
    assert_eq!(names::tags_for("Run To Rolling"), ["locomotion", "dodge"]);
    assert_eq!(
        names::tags_for("Standing 1H Magic Attack 01"),
        ["combat", "magic"]
    );
    assert_eq!(names::tags_for("Standing Dodge Left"), ["combat", "dodge"]);
    assert!(names::tags_for("heroine take").is_empty());
    assert_eq!(
        names::clip_name("Standing Dodge Left.fbx"),
        "Standing Dodge Left"
    );
    assert_eq!(
        names::clip_name("heroine_cast_take3.fbx"),
        "heroine cast take3"
    );
    assert_eq!(names::pack_name("Pro Magic Pack.zip"), "Pro Magic Pack");
    assert_eq!(
        names::clean_pack("  Duelist   moves "),
        Some("Duelist moves".into())
    );
    assert_eq!(names::clean_pack("   "), None);
}

fn write_zip(path: &Path, entries: &[&str]) {
    let mut zip = zip::ZipWriter::new(File::create(path).expect("zip"));
    let options = zip::write::SimpleFileOptions::default();
    for name in entries {
        zip.start_file(*name, options).expect("entry");
        zip.write_all(b"fbx").expect("write");
    }
    zip.finish().expect("finish");
}

#[test]
fn staging_unzips_packs_and_leaves_out_what_is_not_fbx() {
    let from = tempfile::tempdir().expect("tempdir");
    let work = tempfile::tempdir().expect("tempdir");
    let zip = from.path().join("Pro Magic Pack.zip");
    write_zip(
        &zip,
        &[
            "Pro Magic Pack/Cast Spell.fbx",
            "__MACOSX/._Cast Spell.fbx",
            "readme.txt",
            "Pro Magic Pack/Area Attack.FBX",
        ],
    );
    let single = from.path().join("Standing Dodge Left.fbx");
    fs::write(&single, b"fbx").expect("fbx");
    let notes = from.path().join("notes.txt");
    fs::write(&notes, b"hi").expect("notes");
    let gone = from.path().join("gone.fbx");
    let paths = [&zip, &single, &notes, &gone].map(|p| p.display().to_string());
    let staged = import::stage_for_tests(work.path(), &paths).expect("stage");
    assert_eq!(staged.len(), 4);
    let inner: Vec<&str> = staged[0].entries.iter().map(|e| e.file.as_str()).collect();
    assert_eq!(inner, ["Area Attack.FBX", "Cast Spell.fbx"]);
    assert!(staged[0].archive);
    assert!(work.path().join(&staged[0].entries[0].path).is_file());
    assert_eq!(staged[1].entries[0].file, "Standing Dodge Left.fbx");
    assert_eq!(
        staged[2].left_out[0].reason,
        "it is not an FBX file or a zip of them"
    );
    assert_eq!(staged[3].left_out[0].reason, "the file is not there");
    assert!(import::stage_for_tests(work.path(), &[]).is_err());
}

fn sample() -> (Vec<StagedFile>, ReadReport) {
    let staged = vec![
        pack(
            0,
            "Pro Sword and Shield Pack.zip",
            &["Sword And Shield Idle.fbx", "Sword And Shield Slash.fbx"],
        ),
        pack(
            1,
            "Pro Magic Pack.zip",
            &["Standing 1H Magic Attack 01.fbx", "magic_pose.fbx"],
        ),
        single(2, "Standing Dodge Left.fbx"),
        single(3, "heroine_cast_take3.fbx"),
    ];
    let custom: Vec<String> = (0..58).map(|n| format!("heroine_bone_{n}")).collect();
    let mixamo = mixamo_bones();
    let report = ReadReport {
        files: vec![
            file_report("0-0", &mixamo, vec![clip_report("idle", true)]),
            file_report("0-1", &mixamo, vec![clip_report("slash", true)]),
            file_report("1-0", &mixamo, vec![clip_report("cast", true)]),
            file_report("1-1", &mixamo, vec![clip_report("pose", false)]),
            file_report("2-0", &mixamo, vec![clip_report("dodge2", true)]),
            file_report("3-0", &custom, vec![clip_report("heroine", true)]),
        ],
    };
    (staged, report)
}

fn library_sample() -> [AnimationClip; 2] {
    [
        library_clip("idle", "Sword And Shield Idle", SINGLES, "idle"),
        library_clip("dodge", "Standing Dodge Left", SINGLES, "dodge1"),
    ]
}

#[test]
fn an_import_finds_new_clips_duplicates_revisions_and_files_without_keys() {
    let (staged, report) = sample();
    let plan = import::classify("import-1", &staged, &report, &library_sample(), None);
    let sword = &plan.files[0];
    assert_eq!(sword.pack, "Pro Sword and Shield Pack");
    assert_eq!(sword.rig, Some(SourceRig::Mixamo));
    assert_eq!(sword.prefix.as_deref(), Some("mixamorig:"));
    assert_eq!((sword.bones, sword.fps, sword.skinned), (22, Some(30.0), 2));
    assert_eq!(sword.clips[0].status, ClipStatus::Duplicate);
    let matched = sword.clips[0].matched.as_ref().expect("matched");
    assert_eq!(matched.pack, SINGLES);
    assert_eq!(sword.clips[1].status, ClipStatus::New);
    assert_eq!(sword.clips[1].tags, ["combat"]);
    let magic = &plan.files[1];
    assert_eq!(magic.clips.len(), 1);
    assert_eq!(magic.left_out[0].file, "magic_pose.fbx");
    assert_eq!(magic.left_out[0].reason, "it holds one pose and no keys");
    let dodge = &plan.files[2];
    assert_eq!(dodge.pack, SINGLES);
    assert_eq!(dodge.clips[0].status, ClipStatus::Revision);
    let heroine = &plan.files[3];
    assert_eq!(heroine.rig, Some(SourceRig::Custom));
    assert_eq!(heroine.clips[0].name, "heroine cast take3");
    assert_eq!((plan.add, plan.left_out), (4, 1));
    assert_eq!(plan.stored_bytes, 4.0 * 46_000.0);
    assert_eq!(plan.tags[0].tag, "combat");
}

#[test]
fn the_same_keys_twice_in_one_import_go_in_once() {
    let staged = vec![
        single(0, "Run To Rolling.fbx"),
        single(1, "Run To Rolling copy.fbx"),
    ];
    let report = ReadReport {
        files: vec![
            file_report("0-0", &mixamo_bones(), vec![clip_report("roll", true)]),
            file_report("1-0", &mixamo_bones(), vec![clip_report("roll", true)]),
        ],
    };
    let plan = import::classify("import-1", &staged, &report, &[], None);
    let second = &plan.files[1].clips[0];
    assert_eq!(second.status, ClipStatus::Duplicate);
    assert_eq!(second.matched.as_ref().and_then(|m| m.id.clone()), None);
    assert_eq!(plan.add, 1);
}

#[test]
fn the_request_renames_packs_keeps_both_and_skips() {
    let (staged, report) = sample();
    let request = ImportRequest {
        token: "import-1".into(),
        packs: vec![PackChoice {
            file: "Pro Magic Pack.zip".into(),
            pack: "  Spells ".into(),
        }],
        singles: Some("Loose takes".into()),
        keep_both: vec!["0-0/0".into()],
        skip: vec!["3-0/0".into()],
        ..ImportRequest::default()
    };
    let existing = library_sample();
    let plan = import::classify("import-1", &staged, &report, &existing, Some(&request));
    assert_eq!(plan.files[1].pack, "Spells");
    assert_eq!(plan.files[2].pack, "Loose takes");
    // Under another pack the dodge is a new clip, not a revision.
    assert_eq!(plan.files[2].clips[0].status, ClipStatus::New);
    assert!(import::goes_in(&plan.files[0].clips[0], Some(&request)));
    assert!(!import::goes_in(&plan.files[3].clips[0], Some(&request)));
    assert_eq!(plan.add, 4);
}

/// A read import in a workspace's cache, as Blender would leave it.
fn read_import(root: &Path, config: &WorkspaceConfig) -> String {
    let (staged, report) = sample();
    let token = "import-7".to_string();
    let work = root
        .join(&config.layout.cache)
        .join(ANIMATIONS_DIR)
        .join(&token);
    for clip in report.files.iter().flat_map(|f| &f.clips) {
        let Some(out) = &clip.out else { continue };
        let path = work.join(blender::OUT_DIR).join(out);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(path, format!("glb {}", clip.hash)).expect("glb");
    }
    for entry in staged.iter().flat_map(|f| &f.entries) {
        let path = work.join(&entry.path);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(path, b"fbx").expect("fbx");
    }
    crate::json::write_json(&work.join("staged.json"), &staged).expect("staged");
    let files: Vec<serde_json::Value> = report.files.iter().map(report_json).collect();
    let json = serde_json::json!({ "files": files });
    crate::json::write_json(&work.join(blender::REPORT_FILE), &json).expect("report");
    token
}

fn report_json(f: &FileReport) -> serde_json::Value {
    let bones: Vec<serde_json::Value> = f
        .bones
        .iter()
        .map(|b| serde_json::json!({ "name": b.name, "parent": b.parent, "head": b.head, "tail": b.tail }))
        .collect();
    let clips: Vec<serde_json::Value> = f
        .clips
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.name, "hash": c.hash, "hasKeys": c.has_keys, "frames": c.frames,
                "fps": c.fps, "length": c.length, "rootMotion": c.root_motion, "loops": c.loops,
                "bytes": c.bytes, "out": c.out,
            })
        })
        .collect();
    serde_json::json!({
        "id": f.id, "error": f.error, "skinned": f.skinned, "hipsHeight": f.hips_height,
        "bones": bones, "clips": clips,
    })
}

#[test]
fn commit_adds_the_chosen_clips_and_revises_a_fixed_one() {
    let root = tempfile::tempdir().expect("tempdir");
    let config = WorkspaceConfig::default();
    keep(
        root.path(),
        "sword-and-shield-idle",
        "Sword And Shield Idle",
        SINGLES,
        "idle",
    );
    keep(
        root.path(),
        "standing-dodge-left",
        "Standing Dodge Left",
        SINGLES,
        "dodge1",
    );
    let token = read_import(root.path(), &config);
    let request = ImportRequest {
        token: token.clone(),
        keep_files: true,
        ..ImportRequest::default()
    };
    let done = import::commit(root.path(), &config, &request).expect("commit");
    let added: Vec<&str> = done.added.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        added,
        [
            "Sword And Shield Slash",
            "Standing 1H Magic Attack 01",
            "heroine cast take3"
        ]
    );
    assert_eq!((done.skipped, done.left_out), (1, 1));
    let revised = &done.revised[0];
    assert_eq!(revised.id, "standing-dodge-left");
    assert_eq!((revised.revision, revised.hash.as_str()), (2, "dodge2"));
    let slash = load(root.path(), "sword-and-shield-slash").expect("slash");
    assert_eq!(slash.pack, "Pro Sword and Shield Pack");
    assert_eq!(
        slash.origin.archive.as_deref(),
        Some("Pro Sword and Shield Pack.zip")
    );
    assert_eq!(slash.origin.kept.as_deref(), Some("source.fbx"));
    assert_eq!(slash.rig.kind, SourceRig::Mixamo);
    let folder = clip_dir(root.path(), "sword-and-shield-slash").expect("dir");
    let glb = fs::read_to_string(folder.join(CLIP_GLB)).expect("glb");
    assert_eq!(glb, "glb slash");
    assert!(folder.join("source.fbx").is_file());
    let custom = load(root.path(), "heroine-cast-take3").expect("custom");
    assert_eq!(custom.rig.kind, SourceRig::Custom);
    let work = root.path().join(&config.layout.cache).join(ANIMATIONS_DIR);
    assert!(!work.join(&token).exists());
    assert!(import::commit(root.path(), &config, &request).is_err());
}

#[test]
fn commit_keeps_both_when_asked_and_drops_the_fbx_by_default() {
    let root = tempfile::tempdir().expect("tempdir");
    let config = WorkspaceConfig::default();
    keep(
        root.path(),
        "sword-and-shield-idle",
        "Sword And Shield Idle",
        SINGLES,
        "idle",
    );
    let token = read_import(root.path(), &config);
    let request = ImportRequest {
        token,
        keep_both: vec!["0-0/0".into()],
        ..ImportRequest::default()
    };
    let done = import::commit(root.path(), &config, &request).expect("commit");
    assert!(done.added.iter().any(|c| c.id == "sword-and-shield-idle-2"));
    let dodge = load(root.path(), "standing-dodge-left").expect("dodge");
    assert_eq!((dodge.revision, dodge.origin.kept), (1, None));
    let bad = ImportRequest {
        token: "../x".into(),
        ..ImportRequest::default()
    };
    assert!(import::commit(root.path(), &config, &bad).is_err());
}
