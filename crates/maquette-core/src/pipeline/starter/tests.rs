use super::*;
use crate::pipeline::template::for_template;

#[test]
fn every_template_script_has_a_starter() {
    for template in [
        ProjectTemplate::FloatingDiorama,
        ProjectTemplate::Room,
        ProjectTemplate::CharacterShowcase,
    ] {
        let def = for_template(template);
        for step in &def.steps {
            if let Some(script) = &step.script {
                assert!(
                    scripts(template).iter().any(|s| s.path == script),
                    "{template:?} has no starter for {script}"
                );
            }
        }
    }
    assert!(scripts(ProjectTemplate::Imported).is_empty());
}

#[test]
fn starters_stay_plain_and_headless() {
    for script in scripts(ProjectTemplate::FloatingDiorama)
        .iter()
        .chain(scripts(ProjectTemplate::CharacterShowcase))
    {
        assert!(
            !script.text.contains('\u{2014}'),
            "{} has an em dash",
            script.path
        );
        assert!(
            !script.text.contains('\u{2013}'),
            "{} has an en dash",
            script.path
        );
        // The window is never opened from a pipeline script.
        assert!(!script.text.contains("wm.window"), "{}", script.path);
    }
}

#[test]
fn names_blender_renamed_on_import_ship_as_the_model_has_them() {
    let build = BUILD_SCENE.text;
    assert!(build.contains("SOURCE_NAME = \"maquette_name\""));
    assert!(build.contains("keep_source_names(path, added)"));
    let export = EXPORT_WEB.text;
    assert!(export.contains("SOURCE_NAME = \"maquette_name\""));
    // The tags come off before any export, so they never ship as extras.
    assert!(export.contains("del item[SOURCE_NAME]"));
    assert!(export.contains("undo_names(undo)"));
}

#[test]
fn posters_frame_the_scene_when_it_has_no_stops_and_never_save_it() {
    let text = RENDER_STOPS.text;
    assert!(text.contains("def overview_camera(scene)"));
    assert!(text.contains("one framed three quarter poster of the whole scene"));
    assert!(text.contains("remove_camera(camera)"));
    // The temporary camera must not end up in the user's blend.
    assert!(!text.contains("save_mainfile") && !text.contains("save_as_mainfile"));
}

#[test]
fn the_build_places_models_from_the_layout_and_posters_keep_its_names() {
    let build = BUILD_SCENE.text;
    assert!(build.contains("--layout"));
    assert!(build.contains("layout.json"));
    assert!(build.contains("def place(collection, placement)"));
    assert!(build.contains("def add_stops(scene, stops)"));
    assert!(build.contains("\"maquette_stop_name\""));
    // Cameras the user placed in the base blend win over the layout's.
    assert!(build.contains("own_stops(scene)"));
    let posters = RENDER_STOPS.text;
    assert!(posters.contains("LABEL = \"maquette_stop_name\""));
    assert!(posters.contains("entry[\"label\"]"));
}

#[test]
fn the_build_prepares_models_before_the_layout_places_them() {
    let build = BUILD_SCENE.text;
    assert!(build.contains("--prepare"));
    assert!(build.contains("prepare.json"));
    let prepared = build.find("prepare_models(added").expect("prepares");
    let placed = build
        .find("place(collection, placement)\n")
        .expect("places");
    assert!(
        prepared < placed,
        "real size first, the layout's scale on top"
    );
}

#[test]
fn the_fixup_script_applies_the_kinds_in_their_order() {
    let text = FIXUPS.text;
    let names: Vec<String> = crate::prepare::FixupKind::ALL
        .iter()
        .map(|k| serde_json::to_value(k).expect("kind"))
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let list = text
        .split("ORDER = [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("ORDER");
    let order: Vec<String> = list
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect();
    assert_eq!(order, names, "the script and FixupKind agree");
    for name in &names {
        assert!(
            text.contains(&format!("\"{name}\": ")),
            "{name} has a function"
        );
    }
    // It reads its input and writes a copy; it never saves a blend.
    assert!(!text.contains("save_mainfile") && !text.contains("save_as_mainfile"));
    assert!(text.contains("export_scene.gltf(filepath=out"));
}

#[test]
fn write_skips_files_that_exist() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = write(dir.path(), ProjectTemplate::FloatingDiorama).expect("write");
    assert_eq!(first.len(), 6);
    let character = tempfile::tempdir().expect("tempdir");
    let written = write(character.path(), ProjectTemplate::CharacterShowcase).expect("write");
    assert_eq!(written.len(), 4);
    assert!(written.contains(&"tools/build_character.py".to_string()));
    let edited = dir.path().join("tools/build_scene.py");
    fs::write(&edited, "print('mine')\n").expect("edit");
    let again = write(dir.path(), ProjectTemplate::FloatingDiorama).expect("again");
    assert!(again.is_empty());
    assert_eq!(
        fs::read_to_string(&edited).expect("read"),
        "print('mine')\n"
    );
}

fn state(project: &Path, path: &str) -> StarterState {
    status(project, ProjectTemplate::FloatingDiorama)
        .into_iter()
        .find(|s| s.path == path)
        .expect("listed")
        .state
}

#[test]
fn fresh_starters_are_recorded_and_current() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), ProjectTemplate::FloatingDiorama).expect("write");
    assert!(dir.path().join(RECORD_FILE).is_file());
    assert!(
        status(dir.path(), ProjectTemplate::FloatingDiorama)
            .iter()
            .all(|s| s.state == StarterState::Current)
    );
    assert!(status(dir.path(), ProjectTemplate::Imported).is_empty());
}

#[test]
fn a_recorded_copy_of_an_older_starter_updates_and_an_edited_one_waits() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path();
    // As if an older Maquette had written these two and recorded them.
    let old_bake = "print('stub: nothing baked')\n";
    let old_export = "print('old export')\n";
    fs::create_dir_all(project.join("tools")).expect("tools");
    fs::write(project.join(BAKE_LIGHT.path), old_bake).expect("bake");
    fs::write(project.join(EXPORT_WEB.path), old_export).expect("export");
    let mut record = Record::default();
    record
        .scripts
        .insert(BAKE_LIGHT.path.into(), text_hash(old_bake));
    record
        .scripts
        .insert(EXPORT_WEB.path.into(), text_hash(old_export));
    save_record(project, &record).expect("record");
    // The user changed the export afterwards.
    fs::write(project.join(EXPORT_WEB.path), "print('mine')\n").expect("edit");

    assert_eq!(state(project, BAKE_LIGHT.path), StarterState::Outdated);
    assert_eq!(state(project, EXPORT_WEB.path), StarterState::Edited);
    assert_eq!(state(project, BUILD_SCENE.path), StarterState::Missing);

    let all: Vec<String> = scripts(ProjectTemplate::FloatingDiorama)
        .iter()
        .map(|s| s.path.to_string())
        .collect();
    let after = update(project, ProjectTemplate::FloatingDiorama, &all, false).expect("update");
    let of = |path: &str| {
        after
            .iter()
            .find(|s| s.path == path)
            .expect("listed")
            .clone()
    };
    assert_eq!(of(BAKE_LIGHT.path).state, StarterState::Current);
    assert_eq!(of(BUILD_SCENE.path).state, StarterState::Current);
    assert_eq!(of(EXPORT_WEB.path).state, StarterState::Edited);
    assert_eq!(
        fs::read_to_string(project.join(EXPORT_WEB.path)).expect("read"),
        "print('mine')\n",
        "an edited script is never overwritten silently"
    );
    assert_eq!(
        fs::read_to_string(project.join(BAKE_LIGHT.path)).expect("read"),
        BAKE_LIGHT.text
    );

    let replaced = update(
        project,
        ProjectTemplate::FloatingDiorama,
        &[EXPORT_WEB.path.to_string()],
        true,
    )
    .expect("replace");
    let export = replaced
        .iter()
        .find(|s| s.path == EXPORT_WEB.path)
        .expect("listed");
    assert_eq!(export.state, StarterState::Current);
    assert_eq!(export.backup.as_deref(), Some("tools/export_web.py.bak"));
    assert_eq!(
        fs::read_to_string(project.join("tools/export_web.py.bak")).expect("backup"),
        "print('mine')\n"
    );
    // A second replace keeps the first backup.
    fs::write(project.join(EXPORT_WEB.path), "print('mine again')\n").expect("edit");
    let again = update(
        project,
        ProjectTemplate::FloatingDiorama,
        &[EXPORT_WEB.path.to_string()],
        true,
    )
    .expect("again");
    assert_eq!(
        again
            .iter()
            .find(|s| s.path == EXPORT_WEB.path)
            .and_then(|s| s.backup.clone())
            .as_deref(),
        Some("tools/export_web.py.bak2")
    );
}

#[test]
fn unrecorded_scripts_match_the_known_older_starters() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path();
    write(project, ProjectTemplate::FloatingDiorama).expect("write");
    // Before the record existed: no tools/.starter.json.
    fs::remove_file(project.join(RECORD_FILE)).expect("no record");
    // The first bake starter, a stub, as projects from then have it.
    let first = include_str!("../fixtures/bake_light_first.py");
    fs::write(project.join(BAKE_LIGHT.path), first).expect("first");
    assert_eq!(state(project, BAKE_LIGHT.path), StarterState::Outdated);
    let older = "print('a starter from long ago')\n";
    fs::write(project.join(BAKE_LIGHT.path), older).expect("older");
    assert_eq!(
        state(project, BAKE_LIGHT.path),
        StarterState::Edited,
        "unknown text without a record may be edited"
    );
    assert!(
        OLDER.iter().all(|(path, hash)| {
            scripts(ProjectTemplate::FloatingDiorama)
                .iter()
                .chain(scripts(ProjectTemplate::CharacterShowcase))
                .any(|s| s.path == *path && text_hash(s.text) != *hash)
        }),
        "a known older hash is never the current starter"
    );
    // A CRLF copy of the shipped starter still counts as current.
    fs::write(
        project.join(BUILD_SCENE.path),
        BUILD_SCENE.text.replace('\n', "\r\n"),
    )
    .expect("crlf");
    assert_eq!(state(project, BUILD_SCENE.path), StarterState::Current);
}
