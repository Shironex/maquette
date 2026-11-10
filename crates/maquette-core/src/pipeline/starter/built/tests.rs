//! The helper file for built asset scripts and its example: what they
//! promise, and how a starter update reaches the assets that list the
//! helpers. `blender_tests` builds the example in a real Blender.

use std::fs;
use std::path::PathBuf;

use super::super::{
    RECORD_FILE, StarterState, load_record, save_record, status, text_hash, update, write,
};
use super::{BUILT_EXAMPLE, BUILT_HELPERS};
use crate::assets::{self, AssetKind};
use crate::built::{self, BuildRecord, BuiltState, NewBuilt};
use crate::project::{self, NewProject, ProjectTemplate};
use crate::workspace::{self, WorkspaceConfig};

pub(super) struct Fixture {
    _dir: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) config: WorkspaceConfig,
    pub(super) project: String,
    pub(super) project_dir: PathBuf,
}

/// A workspace with one project of `template` and its starters written.
pub(super) fn fixture(template: ProjectTemplate) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("ws");
    let config = workspace::create(&root, &WorkspaceConfig::default()).expect("workspace");
    let manifest = project::create(
        &root,
        &config.layout,
        NewProject {
            name: "Moon Island".into(),
            template,
            target: Default::default(),
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    let project_dir = project::dir(&root, &config.layout, &manifest.id).expect("dir");
    write(&project_dir, template).expect("starters");
    Fixture {
        _dir: dir,
        root,
        config,
        project: manifest.id,
        project_dir,
    }
}

/// Adds a built asset whose files are named relative to the project folder.
pub(super) fn add_built(f: &Fixture, name: &str, script: &str, helpers: &[&str]) -> String {
    let new = NewBuilt {
        name: name.into(),
        kind: AssetKind::Prop,
        script: script.into(),
        helpers: helpers.iter().map(|h| (*h).to_string()).collect(),
        root: None,
        replace: false,
    };
    built::add(&f.root, &f.config.layout, &f.project, &new)
        .expect("add")
        .0
        .id
}

pub(super) fn built_state(f: &Fixture, id: &str) -> BuiltState {
    let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
    built::list(&f.project_dir, &manifest, &[])
        .expect("list")
        .into_iter()
        .find(|b| b.asset == id)
        .expect("a built asset")
        .state
}

/// The public helpers: the top level functions with no underscore in front.
fn helpers() -> Vec<&'static str> {
    BUILT_HELPERS
        .text
        .lines()
        .filter_map(|line| line.strip_prefix("def "))
        .filter(|rest| !rest.starts_with('_'))
        .filter_map(|rest| rest.split('(').next())
        .collect()
}

#[test]
fn every_helper_has_a_docstring_and_the_example_uses_it() {
    let names = helpers();
    for wanted in [
        "page_block",
        "lathe",
        "crisp_edges",
        "mesh_object",
        "brass",
        "glazed_ceramic",
        "join",
        "finish",
    ] {
        assert!(names.contains(&wanted), "{wanted} is gone: {names:?}");
    }
    for name in &names {
        let at = BUILT_HELPERS
            .text
            .find(&format!("\ndef {name}("))
            .expect("the definition");
        let rest = &BUILT_HELPERS.text[at..];
        let body = &rest[rest.find("):\n").expect("the end of the signature") + 3..];
        assert!(
            body.trim_start().starts_with("\"\"\""),
            "{name} has no docstring"
        );
        assert!(
            BUILT_EXAMPLE.text.contains(&format!("mq.{name}(")),
            "the example does not use {name}"
        );
    }
    // The example imports the helper file by the name it ships under.
    let module = BUILT_HELPERS
        .path
        .trim_start_matches("built/")
        .trim_end_matches(".py");
    assert!(
        BUILT_EXAMPLE
            .text
            .contains(&format!("import {module} as mq"))
    );
}

#[test]
fn every_kind_of_project_gets_the_two_built_files() {
    // A website from any template, an imported folder and a game can all
    // hold a built asset, so each lists the two files after its own scripts.
    let dir = tempfile::tempdir().expect("tempdir");
    for template in [
        ProjectTemplate::FloatingDiorama,
        ProjectTemplate::Room,
        ProjectTemplate::CharacterShowcase,
        ProjectTemplate::Imported,
        ProjectTemplate::GameImport,
    ] {
        let listed: Vec<String> = status(dir.path(), template)
            .into_iter()
            .map(|s| s.path)
            .collect();
        assert!(
            listed.ends_with(&[
                BUILT_HELPERS.path.to_string(),
                BUILT_EXAMPLE.path.to_string()
            ]),
            "{template:?}: {listed:?}"
        );
    }
    assert_eq!(BUILT_HELPERS.path, "built/maquette_helpers.py");
    assert_eq!(BUILT_EXAMPLE.path, "built/maquette_example.py");
}

#[test]
fn an_imported_project_gets_the_built_files_and_their_record() {
    // No step starters, so nothing made `tools/` for the record before.
    let dir = tempfile::tempdir().expect("tempdir");
    let written = write(dir.path(), ProjectTemplate::Imported).expect("write");
    assert_eq!(written, [BUILT_HELPERS.path, BUILT_EXAMPLE.path]);
    assert!(dir.path().join(RECORD_FILE).is_file());
    assert_eq!(
        fs::read_to_string(dir.path().join(BUILT_HELPERS.path)).expect("helpers"),
        BUILT_HELPERS.text
    );

    // A project made before them: the update writes both.
    let older = tempfile::tempdir().expect("tempdir");
    let missing: Vec<String> = status(older.path(), ProjectTemplate::Imported)
        .into_iter()
        .filter(|s| s.state == StarterState::Missing)
        .map(|s| s.path)
        .collect();
    assert_eq!(missing, [BUILT_HELPERS.path, BUILT_EXAMPLE.path]);
    let after = update(older.path(), ProjectTemplate::Imported, &missing, false).expect("update");
    assert!(after.iter().all(|s| s.state == StarterState::Current));
    assert!(older.path().join(BUILT_EXAMPLE.path).is_file());
}

/// Records a good build of `id` on its files as they are now.
fn mark_built(f: &Fixture, id: &str) {
    let source = built::load(&f.project_dir).expect("built").assets[id].clone();
    let record = BuildRecord {
        key: built::key(&f.project_dir, &source).expect("key"),
        at: project::now_rfc3339(),
        version: Some(1),
        error: None,
        objects: Vec::new(),
        materials: Vec::new(),
        triangles: 0,
        warnings: Vec::new(),
    };
    built::update(&f.project_dir, |file| {
        file.assets.get_mut(id).expect("source").last = Some(record.clone());
        Ok(())
    })
    .expect("record");
    project::modify(&f.root, &f.config.layout, &f.project, |m| {
        let file = format!("models/{}_v1.glb", id.replace('-', "_"));
        assets::add_version(m, id, file, None).map(|_| ())
    })
    .expect("version");
}

#[test]
fn a_starter_update_shows_the_assets_that_list_the_helpers_as_stale() {
    let f = fixture(ProjectTemplate::Room);
    let helpers = f.project_dir.join(BUILT_HELPERS.path);
    // As an older Maquette wrote the helper file, untouched since.
    let older = format!("{}\n# an older helper file\n", BUILT_HELPERS.text);
    fs::write(&helpers, &older).expect("older");
    let mut record = load_record(&f.project_dir);
    record
        .scripts
        .insert(BUILT_HELPERS.path.into(), text_hash(&older));
    save_record(&f.project_dir, &record).expect("record");

    // The mug imports the shipped helpers; the lamp only a file of the project's own.
    for (file, text) in [
        ("built/mug.py", "import maquette_helpers as mq\n"),
        ("built/lamp.py", "import common\n"),
        ("built/common.py", "PALETTE = 1\n"),
    ] {
        fs::write(f.project_dir.join(file), text).expect("script");
    }
    let mug = add_built(&f, "Mug", "built/mug.py", &[BUILT_HELPERS.path]);
    let lamp = add_built(&f, "Lamp", "built/lamp.py", &["built/common.py"]);
    // Listing the shipped file copies nothing: it already is where built scripts live.
    let listed = built::load(&f.project_dir).expect("built").assets[&mug].clone();
    assert_eq!(listed.helpers, [BUILT_HELPERS.path]);
    assert_eq!(fs::read_to_string(&helpers).expect("read"), older);

    mark_built(&f, &mug);
    mark_built(&f, &lamp);
    assert_eq!(built_state(&f, &mug), BuiltState::Current);
    assert_eq!(built_state(&f, &lamp), BuiltState::Current);

    let asked = [BUILT_HELPERS.path.to_string()];
    update(&f.project_dir, ProjectTemplate::Room, &asked, false).expect("update");
    assert_eq!(
        fs::read_to_string(&helpers).expect("read"),
        BUILT_HELPERS.text
    );
    // What the mug builds from changed, so it says so; the lamp never saw it.
    assert_eq!(built_state(&f, &mug), BuiltState::Stale);
    assert_eq!(built_state(&f, &lamp), BuiltState::Current);
    let manifest = project::load(&f.root, &f.config.layout, &f.project).expect("manifest");
    let again: Vec<String> = built::to_rebuild(&f.project_dir, &manifest)
        .expect("to rebuild")
        .into_iter()
        .map(|b| b.asset)
        .collect();
    assert_eq!(again, [mug]);
    assert_eq!(
        fs::read_to_string(f.project_dir.join("built/common.py")).expect("own"),
        "PALETTE = 1\n",
        "a project's own helper file is never touched"
    );
}
