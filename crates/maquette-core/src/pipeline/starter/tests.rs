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
    // Edge padding is the asset's padEdges switch, not a fix-up kind: it
    // runs on the model's own UVs, before a decimate moves them.
    let pad = order
        .iter()
        .position(|o| o == "pad-edges")
        .expect("pad-edges");
    assert_eq!(order[pad + 1], "decimate");
    assert!(text.contains(r#"if entry.get("padEdges") is True:"#));
    let kinds: Vec<String> = order
        .iter()
        .filter(|o| *o != "pad-edges")
        .cloned()
        .collect();
    assert_eq!(kinds, names, "the script and FixupKind agree");
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
                .chain(scripts(ProjectTemplate::GameImport))
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

#[test]
fn assets_set_to_skip_the_bake_get_no_lightmap_and_no_second_uv() {
    let tag = "SKIP_BAKE = \"maquette_skip_bake\"";
    let build = BUILD_SCENE.text;
    assert!(build.contains(tag), "Build scene tags them");
    assert!(build.contains("entry.get(\"skipBake\") is True"));
    let bake = BAKE_LIGHT.text;
    assert!(
        bake.contains("SKIP_TAG = \"maquette_skip_bake\""),
        "the bake reads the same tag"
    );
    assert!(bake.contains("\"skip-bake\""));
    // Skipped meshes never reach ensure_lightmap_uv: only targets do.
    assert!(
        bake.contains("for obj in targets)")
            || bake.contains("ensure_lightmap_uv(obj) for obj in targets")
    );
    let export = EXPORT_WEB.text;
    assert!(export.contains("def drop_lightmap_uv(obj)"));
    assert!(export.contains("o.name not in lit"));
}

#[test]
fn materials_that_glow_keep_their_emission() {
    // Bake light and Export GLB decide "glows" the same way, so a sky the
    // bake leaves out is also never given a lightmap on export.
    let glows = |text: &str| -> String {
        let start = text.find("def glows(material):").expect("glows");
        let body = &text[start..];
        let doc_end = body[3..]
            .find("\"\"\"\n")
            .map(|i| i + 3)
            .expect("docstring");
        let body = &body[doc_end..];
        let doc_close = body.find('\n').expect("line") + 1;
        let body = &body[doc_close..];
        body[..body.find("\n\n\n").expect("end of glows")].to_string()
    };
    assert_eq!(glows(BAKE_LIGHT.text), glows(EXPORT_WEB.text));
    assert!(BAKE_LIGHT.text.contains("\"glows\""));
    assert!(EXPORT_WEB.text.contains("if glows(material):"));
}

#[test]
fn the_bake_denoises_and_its_flags_match_the_bake_settings() {
    let bake = BAKE_LIGHT.text;
    assert!(bake.contains("\"--denoise\""));
    assert!(bake.contains("argparse.BooleanOptionalAction"));
    assert!(bake.contains("CompositorNodeDenoise"));
    let d = crate::pipeline::bake::DEFAULT;
    assert!(bake.contains(&format!("\"--size\", type=int, default={}", d.size)));
    assert!(bake.contains(&format!("\"--samples\", type=int, default={}", d.samples)));
    assert!(bake.contains("default=True,"), "denoise is on by default");
    // The denoise scene is removed again, so it never lands in the blend.
    let closed = bake.find("denoiser.close()").expect("close");
    let saved = bake.rfind("save_blend(args.out_blend)").expect("save");
    assert!(closed < saved);
}

#[test]
fn the_desk_slab_stays_out_of_a_floor_and_the_world_is_lit() {
    let build = BUILD_SCENE.text;
    assert!(build.contains("def floor_under(parent, x, y):"));
    assert!(build.contains("so the layout's desk slab is left out"));
    // Blender 5 renders the world's nodes, not its viewport colour.
    assert!(build.contains("background.inputs[\"Color\"].default_value"));
}

#[test]
fn an_update_refuses_paths_that_are_not_starters_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path();
    let old_bake = "print('old bake')\n";
    fs::create_dir_all(project.join("tools")).expect("tools");
    fs::write(project.join(BAKE_LIGHT.path), old_bake).expect("bake");
    let mut record = Record::default();
    record
        .scripts
        .insert(BAKE_LIGHT.path.into(), text_hash(old_bake));
    save_record(project, &record).expect("record");

    let asked = vec![
        BAKE_LIGHT.path.to_string(),
        "tools/bake.py".to_string(), // a typo, or a path from another template
    ];
    let err = update(project, ProjectTemplate::FloatingDiorama, &asked, false)
        .expect_err("an unknown path is refused");
    assert!(err.to_string().contains("tools/bake.py"), "{err}");
    assert_eq!(
        fs::read_to_string(project.join(BAKE_LIGHT.path)).expect("read"),
        old_bake,
        "nothing is written when a path is refused"
    );
    // A character project has no bake starter at all.
    let err = update(
        project,
        ProjectTemplate::CharacterShowcase,
        &[BAKE_LIGHT.path.to_string()],
        false,
    )
    .expect_err("not a starter of this template");
    assert!(err.to_string().contains("not a starter"), "{err}");
}
