//! The undo: one action brings the old model back, and the project folder
//! is what it was. Rebuilds keep working for the replaced asset in between.

use std::collections::BTreeMap;

use super::*;
use crate::assets::AssetOrigin;
use crate::materials::{MaterialTweak, ModelMaterials};
use crate::prepare::{Fixup, ModelPrep, RealSize, SizeAxis, SizeUnit};

/// The project folder's files, with the one field an undo is allowed to
/// change taken out: when the asset was last touched.
fn snapshot(w: &World, asset: &str) -> BTreeMap<String, Vec<u8>> {
    let mut found = files(&w.dir);
    let text = String::from_utf8(found["project.json"].clone()).expect("utf8");
    let mut manifest: serde_json::Value = serde_json::from_str(&text).expect("json");
    for entry in manifest["assets"].as_array_mut().expect("assets") {
        if entry["id"] == asset {
            entry["updatedAt"] = serde_json::Value::Null;
        }
    }
    found.insert(
        "project.json".into(),
        serde_json::to_vec_pretty(&manifest).expect("json"),
    );
    found
}

/// A scene with everything a replace looks at: a placed scan with a real
/// size, scan fix-ups, a tint and a library origin, a second asset after
/// the built one on the Board, and a built asset with settings of its own.
fn busy_scene() -> (World, Asset, Asset) {
    let w = World::website();
    let planner = w.scan("Spiral planner");
    let desk = w.built("Planner built", "planner.py");
    w.scan("Desk lamp");
    w.place(&planner.id);
    prepare::save_entry(
        &w.dir,
        &planner.id,
        &ModelPrep {
            size: Some(RealSize {
                axis: SizeAxis::Longest,
                value: 30.0,
                unit: SizeUnit::Cm,
            }),
            fixups: vec![Fixup::Decimate { triangles: 60_000 }, Fixup::OriginBottom],
            skip_bake: Some(true),
            pad_edges: Some(true),
            parts: None,
        },
    )
    .expect("scan prep");
    prepare::save_entry(
        &w.dir,
        &desk.id,
        &ModelPrep {
            fixups: vec![Fixup::OriginBottom],
            ..ModelPrep::default()
        },
    )
    .expect("built prep");
    let tint = ModelMaterials {
        all: MaterialTweak {
            tint: Some("#3a3936".into()),
            ..MaterialTweak::default()
        },
        materials: BTreeMap::new(),
    };
    materials::save_looks(&w.dir, &BTreeMap::from([(planner.id.clone(), tint)])).expect("looks");
    project::modify(&w.root, &w.config.layout, &w.project, |m| {
        let asset = assets::find_mut(m, &planner.id)?;
        asset.origin = Some(AssetOrigin {
            entry: "planner".into(),
            version: 1,
        });
        asset.thumbnail = Some("thumbs/spiral_planner_v1.png".into());
        Ok(())
    })
    .expect("origin");
    for thumb in [
        "thumbs/spiral_planner_v1.png",
        "thumbs/planner_built_v1.png",
    ] {
        let path = w.dir.join(thumb);
        fs::create_dir_all(path.parent().expect("thumbs")).expect("thumbs dir");
        fs::write(path, thumb.as_bytes()).expect("thumb");
    }
    (w, planner, desk)
}

#[test]
fn a_replace_followed_by_the_undo_leaves_the_project_files_as_they_were() {
    let (w, planner, desk) = busy_scene();
    let before = snapshot(&w, &planner.id);
    let order: Vec<String> = w.manifest().assets.iter().map(|a| a.id.clone()).collect();

    let replaced = w.replace(&planner.id, &desk.id);
    assert_ne!(
        snapshot(&w, &planner.id),
        before,
        "the replace changed files"
    );
    // The card shows the built model's picture meanwhile.
    assert_eq!(
        w.asset("spiral-planner").thumbnail.as_deref(),
        Some("thumbs/spiral_planner_v2.png")
    );

    let undone = undo(&w.env(), &planner.id).expect("undo");

    let after = snapshot(&w, &planner.id);
    let names = |files: &BTreeMap<String, Vec<u8>>| files.keys().cloned().collect::<Vec<_>>();
    assert_eq!(names(&after), names(&before), "the same files are there");
    for (file, bytes) in &before {
        assert!(
            after[file] == *bytes,
            "{file} differs after the undo:\n{}\nwas\n{}",
            String::from_utf8_lossy(&after[file]),
            String::from_utf8_lossy(bytes)
        );
    }
    // The cards stand where they stood.
    let now: Vec<String> = w.manifest().assets.iter().map(|a| a.id.clone()).collect();
    assert_eq!(now, order);
    // The report says what came back and where the built version went.
    let report = &undone.report;
    assert_eq!(report.back.file, "models/spiral_planner_v1.glb");
    assert_eq!(report.taken_off.len(), 1);
    assert_eq!(report.taken_off[0].file, "models/spiral_planner_v2.glb");
    assert_eq!(report.restores.as_deref(), Some("Planner built"));
    assert_eq!(
        report.settings,
        [
            "Real size, 30 cm on its longest side",
            "Decimate to 60k",
            "Origin to bottom centre",
            "Skips the bake",
            "Pads texture edges",
            "Material tweaks",
            "Library origin, planner v1",
        ]
    );
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    // Nothing was deleted: the built version sits in the workspace trash.
    let aside = w.root.join(report.set_aside.as_deref().expect("set aside"));
    assert!(aside.join("files/models/spiral_planner_v2.glb").is_file());
    assert!(aside.join("files/thumbs/spiral_planner_v2.png").is_file());
    assert!(aside.join("set-aside.json").is_file());
    // The built asset's trash entry is used up.
    let trash = replaced.report.trash.expect("trash id");
    let listed = remove::list_trash(&w.root, &w.config.layout, &w.project).expect("trash");
    assert!(listed.iter().all(|entry| entry.id != trash));
}

#[test]
fn the_undo_plan_reads_only_and_an_asset_that_was_added_as_built_has_none() {
    let (w, planner, desk) = busy_scene();
    let stack = w.built("Book stack", "books.py");
    w.replace(&planner.id, &desk.id);
    let before = files(&w.dir);

    let report = undo_plan(&w.env(), &planner.id).expect("plan");

    assert_eq!(files(&w.dir), before, "a plan writes nothing");
    assert_eq!(report.back.version, 1);
    assert_eq!(report.script, "built/planner.py");
    assert_eq!(report.set_aside, None);
    let plain = undo(&w.env(), &stack.id)
        .expect_err("never replaced")
        .to_string();
    assert!(plain.contains("took no other model's place"), "{plain}");
    let scan = undo(&w.env(), "desk-lamp")
        .expect_err("not built")
        .to_string();
    assert!(scan.contains("Desk lamp is not a built asset"), "{scan}");
}

#[test]
fn an_undo_goes_on_when_the_built_assets_card_cannot_come_back() {
    let (w, planner, desk) = busy_scene();
    let replaced = w.replace(&planner.id, &desk.id);
    // Someone emptied the trash folder by hand.
    let trash = remove::project_trash(&w.root, &w.config.layout, &w.project)
        .expect("trash")
        .join(replaced.report.trash.expect("trash id"));
    fs::remove_dir_all(trash).expect("empty the trash");

    let undone = undo(&w.env(), &planner.id).expect("undo");

    assert_eq!(undone.report.restores, None);
    assert!(
        undone.report.warnings[0].contains("Planner built is not in the asset trash any more"),
        "{:?}",
        undone.report.warnings
    );
    let asset = w.asset("spiral-planner");
    assert_eq!(
        asset.latest().expect("model").file,
        "models/spiral_planner_v1.glb"
    );
    assert!(load(&w.dir).expect("built").assets.is_empty());
    assert!(w.dir.join("built/planner.py").is_file(), "the script stays");
}

#[cfg(unix)]
mod rebuilding {
    use super::*;
    use crate::built::{BuildContext, BuildResult, Builder, to_rebuild};
    use crate::pipeline::tools::{ResolvedTool, Toolbox};
    use crate::toolchain::{HostOs, ToolId};
    use std::ffi::OsString;

    /// A Blender stand in that copies a four box model to `--output`.
    fn fake_blender(w: &World) -> Toolbox {
        let dir = w.root.parent().expect("parent").join("fake");
        fs::create_dir_all(&dir).expect("fake dir");
        let model = dir.join("four.glb");
        fs::write(&model, fixture::split_parts(4)).expect("model");
        let script = format!(
            "#!/bin/sh\n\
             while [ $# -gt 0 ]; do case \"$1\" in --output) o=\"$2\"; shift;; --report) r=\"$2\"; shift;; esac; shift; done\n\
             cp '{model}' \"$o\"\n\
             echo '{{\"objects\": [\"planner\"], \"materials\": [\"Grey\"], \"triangles\": 48}}' > \"$r\"\n",
            model = model.display(),
        );
        let program = dir.join("blender");
        fs::write(&program, script).expect("script");
        let mut perms = fs::metadata(&program).expect("meta").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        fs::set_permissions(&program, perms).expect("chmod");
        let mut tools = Toolbox {
            path_env: OsString::from("/bin:/usr/bin"),
            os: Some(HostOs::current()),
            ..Toolbox::default()
        };
        tools.tools.insert(
            ToolId::Blender,
            ResolvedTool {
                program,
                prefix: Vec::new(),
                version: "5.2.2".into(),
            },
        );
        tools
    }

    #[test]
    fn a_stale_script_rebuilds_the_replaced_asset_and_the_undo_takes_every_build_off() {
        let (w, planner, desk) = busy_scene();
        let before = snapshot(&w, &planner.id);
        w.replace(&planner.id, &desk.id);
        let tools = fake_blender(&w);
        let ctx = BuildContext {
            workspace_root: &w.root,
            config: &w.config,
            project: &w.project,
            tools: &tools,
        };
        let builder = Builder::default();

        // Up to date right after the replace: no Blender run, nothing to rebuild.
        let same = builder
            .build(&ctx, &planner.id, false, &|_| {})
            .expect("build");
        assert_eq!(same.result, BuildResult::UpToDate);
        assert!(to_rebuild(&w.dir, &w.manifest()).expect("list").is_empty());

        // An edit to the script makes the replaced asset stale, as a run finds it.
        fs::write(w.dir.join("built/planner.py"), "# a thicker cover\n").expect("edit");
        let stale = to_rebuild(&w.dir, &w.manifest()).expect("list");
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].asset, "spiral-planner");
        let rebuilt = builder
            .build(&ctx, &planner.id, false, &|_| {})
            .expect("rebuild");
        assert_eq!(rebuilt.result, BuildResult::Built);
        assert_eq!(
            rebuilt.version.expect("v3").file,
            "models/spiral_planner_v3.glb"
        );
        let source = &load(&w.dir).expect("built").assets["spiral-planner"];
        assert!(source.replaced.is_some(), "a build keeps the record");

        // The undo takes both built versions off and the scan is current again.
        let undone = undo(&w.env(), &planner.id).expect("undo");
        let off: Vec<u32> = undone.report.taken_off.iter().map(|v| v.version).collect();
        assert_eq!(off, [2, 3]);
        let asset = w.asset("spiral-planner");
        assert_eq!(asset.versions.len(), 1);
        // The script was edited meanwhile, so that one file differs.
        let mut after = snapshot(&w, &planner.id);
        let mut was = before;
        assert_ne!(
            after.remove("built/planner.py"),
            was.remove("built/planner.py")
        );
        assert!(after == was, "every other file is as it was");
    }
}
