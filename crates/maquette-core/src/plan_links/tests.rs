use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;
use crate::assets::{AssetStatus, AssetVersion};
use crate::concepts::{ConceptImage, DIRECTION_FILE};
use crate::project::{PROJECT_VERSION, ProjectTemplate};

fn asset(id: &str, versions: &[u32], thumbnail: Option<&str>) -> Asset {
    Asset {
        id: id.into(),
        name: format!("Asset {id}"),
        kind: AssetKind::Character,
        stage: Stage::Blender,
        status: AssetStatus::Done,
        versions: versions
            .iter()
            .map(|n| AssetVersion {
                version: *n,
                file: format!("models/{id}_v{n}.glb"),
                created_at: "2026-11-01T10:00:00.000Z".into(),
                stats: Some(ModelStats {
                    triangles: 18_400,
                    bytes: 2_600_000.0,
                    textures: 1,
                    max_texture: 1024,
                    joints: 0,
                    animations: 0,
                }),
            })
            .collect(),
        thumbnail: thumbnail.map(str::to_string),
        created_at: "2026-11-01T10:00:00.000Z".into(),
        updated_at: "2026-11-01T10:00:00.000Z".into(),
        origin: None,
        refs: Vec::new(),
    }
}

fn manifest(assets: Vec<Asset>) -> ProjectManifest {
    ProjectManifest {
        version: PROJECT_VERSION,
        id: "frostveil".into(),
        name: "Frostveil".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: "2026-10-24T09:30:00.000Z".into(),
        assets,
    }
}

fn touch(dir: &Path, relative: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    fs::write(path, b"x").expect("file");
}

/// An Inbox model made for `asset` of `project` through a send.
fn inbox_for(project: &str, asset: &str, replaced: bool) -> InboxItem {
    serde_json::from_value(json!({
        "id": format!("{asset}.glb"), "name": asset, "file": format!("/inbox/{asset}.glb"),
        "bytes": 10.0, "modifiedMs": 1.0, "kind": "character", "kindGuessed": false,
        "kindFromSend": true, "state": "ready", "report": null, "checks": [], "worst": "pass",
        "error": null, "thumbnail": null, "thumbnailState": "fallback",
        "link": { "job": "job-1", "project": project, "asset": asset, "assetName": asset,
                  "provider": "tripo" },
        "aim": null, "nextVersion": 2, "replacedBy": if replaced { json!(3) } else { json!(null) }
    }))
    .expect("inbox item")
}

fn job(direction: &str, state: ConceptJobState) -> ConceptJob {
    ConceptJob {
        id: format!("{direction}-{state:?}"),
        run_id: None,
        project: "frostveil".into(),
        direction: direction.into(),
        direction_name: direction.into(),
        index: 1,
        file: format!("concepts/{direction}/{direction}_01.png"),
        title: None,
        state,
        queued_at_ms: 0.0,
        started_at_ms: None,
        finished_at_ms: None,
        message: None,
        width: None,
        height: None,
        log_tail: Vec::new(),
        retry_at_ms: None,
        retries: 0,
        usage_limit: false,
        style_revision: None,
        backend: crate::concepts::backend::ImageBackend::Codex,
        refused: false,
    }
}

fn write_direction(dir: &Path, id: &str, picked: &[bool]) {
    let images = picked
        .iter()
        .enumerate()
        .map(|(n, picked)| {
            let file = format!("concepts/{id}/{id}_0{}.png", n + 1);
            touch(dir, &file);
            ConceptImage {
                file,
                index: u32::try_from(n).expect("index") + 1,
                width: None,
                height: None,
                starred: false,
                picked: *picked,
                ref_file: None,
                created_at: "2026-11-01T10:00:00.000Z".into(),
                title: None,
            }
        })
        .collect();
    let direction = Direction {
        id: id.into(),
        name: format!("Direction {id}"),
        images,
        ..Direction::default()
    };
    let folder = dir.join("concepts").join(id);
    fs::create_dir_all(&folder).expect("direction folder");
    fs::write(
        folder.join(DIRECTION_FILE),
        serde_json::to_string(&direction).expect("json"),
    )
    .expect("direction file");
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn each_asset_reads_its_state_live_from_the_project() {
    let dir = tempfile::tempdir().expect("temp dir");
    let project = dir.path();
    touch(project, "models/warden_v2.glb");
    touch(project, "thumbs/warden.webp");
    touch(project, "models/lantern_v1.glb");
    let manifest = manifest(vec![
        asset("warden", &[1, 2], Some("thumbs/warden.webp")),
        asset("gate", &[], Some("thumbs/missing.webp")),
        asset("arch", &[1], None),
        asset("lantern", &[1], None),
    ]);
    let inbox = vec![
        inbox_for("frostveil", "lantern", false),
        inbox_for("frostveil", "warden", true),
        inbox_for("moon-island", "arch", false),
    ];
    let facts = LinkFacts {
        project_dir: project,
        manifest: &manifest,
        inbox: &inbox,
        jobs: &[],
    };
    let links = read(
        &facts,
        &ids(&[
            "warden", "gate", "arch", "lantern", "removed", "../up", "warden",
        ]),
        &[],
    );
    let states: Vec<(&str, AssetLinkState)> = links
        .assets
        .iter()
        .map(|a| (a.id.as_str(), a.state))
        .collect();
    assert_eq!(
        states,
        vec![
            ("warden", AssetLinkState::Ready),
            ("gate", AssetLinkState::NoModel),
            ("arch", AssetLinkState::FileMissing),
            ("lantern", AssetLinkState::Check),
            ("removed", AssetLinkState::Gone),
            ("../up", AssetLinkState::Gone),
        ],
        "each id once, a replaced or other project's model is no check"
    );
    let warden = &links.assets[0];
    assert_eq!(warden.name.as_deref(), Some("Asset warden"));
    assert_eq!(warden.version, Some(2));
    assert_eq!(warden.file.as_deref(), Some("models/warden_v2.glb"));
    assert_eq!(warden.stats.as_ref().map(|s| s.triangles), Some(18_400));
    assert!(
        warden
            .thumbnail
            .as_deref()
            .is_some_and(|t| Path::new(t).ends_with("thumbs/warden.webp"))
    );
    assert_eq!(links.assets[1].thumbnail, None, "a picture not on disk");
    assert_eq!(links.assets[2].file.as_deref(), Some("models/arch_v1.glb"));
    assert_eq!(links.assets[3].checks, 1);
    let gone = &links.assets[4];
    assert_eq!((gone.name.as_deref(), gone.version), (None, None));
}

#[test]
fn a_renamed_asset_shows_its_new_name_on_the_next_read() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut manifest = manifest(vec![asset("warden", &[], None)]);
    let read_name = |m: &ProjectManifest| {
        let facts = LinkFacts {
            project_dir: dir.path(),
            manifest: m,
            inbox: &[],
            jobs: &[],
        };
        read(&facts, &ids(&["warden"]), &[]).assets[0].name.clone()
    };
    assert_eq!(read_name(&manifest).as_deref(), Some("Asset warden"));
    manifest.assets[0].name = "Frost Warden".into();
    assert_eq!(read_name(&manifest).as_deref(), Some("Frost Warden"));
    manifest.assets.clear();
    assert_eq!(read_name(&manifest), None, "removed reads as gone");
}

#[test]
fn each_direction_reads_its_images_picks_and_jobs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let project = dir.path();
    write_direction(project, "kotatsu", &[false, true, false]);
    write_direction(project, "porch", &[false, false]);
    write_direction(project, "lake", &[]);
    write_direction(project, "gate", &[false]);
    let manifest = manifest(Vec::new());
    let jobs = vec![
        job("gate", ConceptJobState::Running),
        job("gate", ConceptJobState::Queued),
        job("gate", ConceptJobState::Done),
        job("porch", ConceptJobState::Done),
    ];
    let facts = LinkFacts {
        project_dir: project,
        manifest: &manifest,
        inbox: &[],
        jobs: &jobs,
    };
    let links = read(
        &facts,
        &[],
        &ids(&["kotatsu", "porch", "lake", "gate", "gone", "con/../x"]),
    );
    let states: Vec<DirectionLinkState> = links.directions.iter().map(|d| d.state).collect();
    assert_eq!(
        states,
        vec![
            DirectionLinkState::Picked,
            DirectionLinkState::Pick,
            DirectionLinkState::Empty,
            DirectionLinkState::Running,
            DirectionLinkState::Gone,
            DirectionLinkState::Gone,
        ]
    );
    let kotatsu = &links.directions[0];
    assert_eq!((kotatsu.images, kotatsu.picked), (3, 1));
    assert!(
        kotatsu
            .thumbnail
            .as_deref()
            .is_some_and(|t| t.ends_with("kotatsu_02.png")),
        "the picked image is the cover"
    );
    assert!(
        links.directions[1]
            .thumbnail
            .as_deref()
            .is_some_and(|t| t.ends_with("porch_02.png")),
        "else the newest"
    );
    assert_eq!(links.directions[1].pending, 0);
    let gate = &links.directions[3];
    assert_eq!((gate.pending, gate.done), (2, 1));
    assert_eq!(links.directions[4].name, None);
}
