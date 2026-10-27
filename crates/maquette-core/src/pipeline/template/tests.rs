use super::*;
use crate::pipeline::graph::{PipelineIssue, validate};
use crate::project::{BlendFile, ImportScan};

fn names(def: &PipelineDef) -> Vec<&str> {
    def.steps.iter().map(|s| s.name.as_str()).collect()
}

#[test]
fn floating_diorama_matches_the_island_flow() {
    let def = for_template(ProjectTemplate::FloatingDiorama);
    assert_eq!(
        names(&def),
        [
            "Build scene",
            "Bake light",
            "Export GLB",
            "Compress",
            "Capture posters",
            "Ship"
        ]
    );
    for step in def.steps.iter().filter(|s| s.kind == StepKind::Blender) {
        assert_eq!(step.args[0], "-b", "{}", step.id);
        assert!(step.args.contains(&"{script}".to_string()));
    }
    // A new project has no blend yet: the build opens none, and every
    // step after it opens the blend the step before it wrote.
    let build = def.step("build").expect("build");
    assert_eq!(
        build.args[..6],
        s(&[
            "-b",
            "--factory-startup",
            "--python-exit-code",
            "1",
            "--python",
            "{script}"
        ])
    );
    for (id, blend) in [
        ("bake", "blender/scene_built.blend"),
        ("export", "blender/scene_baked.blend"),
        ("posters", "blender/scene_baked.blend"),
    ] {
        assert_eq!(def.step(id).expect(id).args[4], blend, "{id}");
        assert!(
            def.steps
                .iter()
                .any(|s| s.outputs.iter().any(|o| o == blend))
        );
    }
    // Outputs every starter script always writes, even with nothing to do.
    assert_eq!(
        def.step("bake").expect("bake").outputs,
        s(&["blender/bake/bake.json", "blender/scene_baked.blend"])
    );
    assert_eq!(
        def.step("posters").expect("posters").outputs,
        s(&["web/posters/stops.json"])
    );
    let ship = def.step("ship").expect("ship");
    let site = ship
        .args
        .iter()
        .position(|a| a == "--site")
        .expect("--site");
    assert_eq!(ship.args[site + 1], "{siteRepo}");
    assert!(!ship.args.iter().any(|a| a.starts_with("{siteRepo}/")));
    let compress = def.step("compress").expect("compress");
    assert_eq!(compress.kind, StepKind::GltfTransform);
    assert!(compress.compress.is_some());
    assert!(def.presets.is_empty(), "no character, no character row");
    assert!(validate(&def).iter().all(|i| !i.is_error()));
}

#[test]
fn the_build_step_reads_the_layout_and_fixups_once_its_script_does() {
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let build = def.step("build").expect("build");
    for input in [LAYOUT_INPUT, PREPARE_INPUT, FIXUPS_INPUT] {
        assert!(build.inputs.iter().any(|i| i == input), "{input}");
    }

    // A pipeline from before layouts, with an older build script.
    let dir = tempfile::tempdir().expect("tempdir");
    let mut old = def.clone();
    old.steps[0]
        .inputs
        .retain(|i| ![LAYOUT_INPUT, PREPARE_INPUT, FIXUPS_INPUT].contains(&i.as_str()));
    std::fs::create_dir_all(dir.path().join("tools")).expect("tools");
    std::fs::write(dir.path().join(BUILD_SCRIPT), "print('no layouts here')\n").expect("old");
    assert!(!fit_build_inputs(&mut old, dir.path()));
    // Updated to the starter that reads layout.json.
    std::fs::write(
        dir.path().join(BUILD_SCRIPT),
        include_str!("../../../starter/build_scene.py"),
    )
    .expect("new");
    assert!(fit_build_inputs(&mut old, dir.path()));
    assert_eq!(old.steps[0].inputs, def.steps[0].inputs);
    assert!(!fit_build_inputs(&mut old, dir.path()), "only once");
}

#[test]
fn a_scene_gets_the_character_row_only_with_a_character() {
    use crate::assets::{Asset, AssetKind, AssetStatus, AssetVersion, Stage};
    let asset = |name: &str, kind: AssetKind, file: &str| Asset {
        id: name.to_lowercase(),
        name: name.into(),
        kind,
        stage: Stage::Blender,
        status: AssetStatus::Review,
        versions: vec![AssetVersion {
            version: 1,
            file: file.into(),
            created_at: String::new(),
            stats: None,
        }],
        thumbnail: None,
        created_at: String::new(),
        updated_at: String::new(),
        origin: None,
    };
    let mut manifest = ProjectManifest {
        version: 1,
        id: "moon-island".into(),
        name: "Moon Island".into(),
        template: ProjectTemplate::FloatingDiorama,
        target: Default::default(),
        game: None,
        site_repo: None,
        site_repo_name: None,
        site_repo_state: Default::default(),
        source_folder: None,
        created_at: String::new(),
        assets: vec![asset("clay", AssetKind::Prop, "models/clay_v1.glb")],
    };
    // An older project: the template row, though it has no character.
    let mut def = for_template(ProjectTemplate::FloatingDiorama);
    def.presets = vec![character_preset()];
    assert!(fit_presets(&mut def, &manifest));
    assert!(def.presets.is_empty());
    assert!(!fit_presets(&mut def, &manifest), "settled");

    // A row the user changed stays.
    let mut edited = def.clone();
    edited.presets = vec![CompressPreset {
        texture_size: 1024,
        ..character_preset()
    }];
    assert!(!fit_presets(&mut edited, &manifest));
    assert_eq!(edited.presets.len(), 1);

    // A character arrives: its export gets the locked row.
    manifest.assets.push(asset(
        "Lantern keeper",
        AssetKind::Character,
        "models/lantern_keeper_v2.glb",
    ));
    assert!(fit_presets(&mut def, &manifest));
    assert_eq!(def.presets.len(), 1);
    assert_eq!(def.presets[0].model, "lantern_keeper_v2");
    assert_eq!(def.presets[0].label, "Lantern keeper");
    assert_eq!(def.presets[0].lock.as_deref(), Some(CHARACTER_LOCK));
    assert!(!fit_presets(&mut def, &manifest));
    // Its v3 is exported as lantern_keeper_v3: the row follows.
    let keeper = manifest.assets.last_mut().expect("keeper");
    keeper.versions.push(AssetVersion {
        version: 3,
        file: "models/lantern_keeper_v3.glb".into(),
        created_at: String::new(),
        stats: None,
    });
    assert!(fit_presets(&mut def, &manifest));
    assert_eq!(def.presets[0].model, "lantern_keeper_v3");

    // A character showcase keeps its own row as it is.
    let mut showcase = for_template(ProjectTemplate::CharacterShowcase);
    manifest.template = ProjectTemplate::CharacterShowcase;
    assert!(!fit_presets(&mut showcase, &manifest));
    assert_eq!(showcase.presets, vec![character_preset()]);
}

#[test]
fn character_showcase_builds_its_own_scene() {
    let def = for_template(ProjectTemplate::CharacterShowcase);
    assert_eq!(
        names(&def),
        [
            "Build character",
            "Export GLB",
            "Compress",
            "Capture posters",
            "Ship"
        ]
    );
    let ids: Vec<&str> = def.steps.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["build", "export", "compress", "posters", "ship"]);
    // A fresh project has only models: the build opens no blend and
    // reads the optional base and the models as inputs.
    let build = def.step("build").expect("build");
    assert_eq!(build.args[4], "--python");
    assert!(build.inputs.contains(&"models/*.glb".to_string()));
    assert_eq!(build.outputs, s(&["blender/character_built.blend"]));
    for id in ["export", "posters"] {
        assert_eq!(
            def.step(id).expect(id).args[4],
            "blender/character_built.blend",
            "{id}"
        );
    }
    assert_eq!(
        def.step("export").expect("export").depends_on,
        s(&["build"])
    );
    assert!(validate(&def).iter().all(|i| !i.is_error()));
}

fn scan_of(blends: &[&str], scripts: &[&str]) -> ImportScan {
    ImportScan {
        path: "/src/island".into(),
        blend_files: blends.iter().map(|b| BlendFile::parse(b)).collect(),
        tool_scripts: scripts.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn import_proposes_steps_from_the_newest_scripts() {
    let scan = scan_of(
        &[
            "models/moon_island_v5.blend",
            "models/moon_island_v5_baked.blend",
            "models/moon_island_v6.blend",
            "models/moon_island_v7.blend",
            "models/moon_island_v7_baked.blend",
            "models/character_web_v4.blend",
        ],
        &[
            "bake_island_light.py",
            "build_island.py",
            "build_island_v6.py",
            "build_island_v7.py",
            "compress_island_web.sh",
            "compress_island_web_v7.sh",
            "contact_sheet.sh",
            "export_character_web.py",
            "export_character_web_v2.py",
            "export_island_web.py",
            "inspect_blend.py",
            "preview_models.py",
            "render_stops.py",
        ],
    );
    let proposal = propose(&scan, Path::new("/src/island"));
    let def = &proposal.pipeline;
    assert_eq!(def.root.as_deref(), Some("/src/island"));
    assert_eq!(
        names(def),
        [
            "Build island",
            "Bake light",
            "Export character",
            "Export island",
            "Compress",
            "Capture posters",
            "Contact sheet"
        ]
    );
    let build = &def.steps[0];
    assert_eq!(build.script.as_deref(), Some("tools/build_island_v7.py"));
    assert_eq!(
        build.inputs,
        vec!["models/moon_island_v6.blend".to_string()]
    );
    assert_eq!(
        build.outputs,
        vec!["models/moon_island_v7.blend".to_string()]
    );
    let bake = &def.steps[1];
    assert!(bake.args.contains(&"models/bake_v7".to_string()));
    assert!(
        bake.outputs
            .contains(&"models/moon_island_v7_baked.blend".to_string())
    );
    let character = &def.steps[2];
    assert_eq!(
        character.inputs,
        vec!["models/character_web_v4.blend".to_string()]
    );
    assert!(character.args.contains(&"models/web_v7".to_string()));
    let export = &def.steps[3];
    assert_eq!(
        export.inputs,
        vec!["models/moon_island_v7_baked.blend".to_string()]
    );
    assert!(export.args.contains(&"models/web_v7".to_string()));
    let compress = &def.steps[4];
    assert_eq!(compress.kind, StepKind::Shell);
    assert_eq!(
        compress.script.as_deref(),
        Some("tools/compress_island_web_v7.sh")
    );
    assert_eq!(
        compress.args,
        vec!["{script}".to_string(), "models/web_v7/dist".to_string()]
    );
    for pair in def.steps.windows(2) {
        assert_eq!(pair[1].depends_on, vec![pair[0].id.clone()]);
    }
    assert_eq!(
        proposal.unused,
        [
            "build_island.py",
            "build_island_v6.py",
            "compress_island_web.sh",
            "export_character_web.py",
            "inspect_blend.py",
            "preview_models.py"
        ]
    );
    let errors: Vec<PipelineIssue> = validate(def).into_iter().filter(|i| i.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    for step in def.steps.iter().filter(|s| s.kind == StepKind::Blender) {
        assert_eq!(step.args[0], "-b");
    }
}

#[test]
fn import_without_blends_proposes_nothing_blender() {
    let proposal = propose(
        &scan_of(&[], &["build_room.py", "notes.sh"]),
        Path::new("/x"),
    );
    assert!(proposal.pipeline.steps.is_empty());
    assert_eq!(proposal.unused, ["build_room.py", "notes.sh"]);
}

#[test]
fn the_ship_step_hands_the_script_the_index_layout_and_tweaks() {
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let ship = def.step("ship").expect("ship");
    let after = |flag: &str| {
        let at = ship.args.iter().position(|a| a == flag).expect(flag);
        ship.args[at + 1].clone()
    };
    assert_eq!(after("--index"), "{shipIndex}");
    assert_eq!(after("--layout"), "{project}/layout.json");
    assert_eq!(after("--materials"), "{project}/materials.json");
    assert!(!ship.args.iter().any(|a| a == "--models-to"));
}

#[test]
fn an_untouched_ship_step_gets_the_new_args_once_its_script_reads_them() {
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let dir = tempfile::tempdir().expect("tempdir");
    let legacy: Vec<String> = [
        "{script}",
        "--models",
        "web/dist",
        "--posters",
        "web/posters",
        "--site",
        "{siteRepo}",
        "--models-to",
        "public/models",
        "--posters-to",
        "public/posters",
    ]
    .iter()
    .map(|a| a.to_string())
    .collect();
    let mut old = def.clone();
    let at = old.steps.iter().position(|s| s.id == "ship").expect("ship");
    old.steps[at].args = legacy.clone();
    let mut edited = old.clone();
    edited.steps[at].args.push("--verbose".into());

    std::fs::create_dir_all(dir.path().join("tools")).expect("tools");
    std::fs::write(dir.path().join("tools/ship.mjs"), "// copies files\n").expect("old");
    assert!(
        !fit_ship_args(&mut old, dir.path()),
        "the old script keeps its args"
    );
    assert_eq!(old.steps[at].args, legacy);

    std::fs::write(
        dir.path().join("tools/ship.mjs"),
        include_str!("../../../starter/ship.mjs"),
    )
    .expect("new");
    assert!(fit_ship_args(&mut old, dir.path()));
    assert_eq!(old.steps[at].args, def.steps[at].args);
    assert!(!fit_ship_args(&mut old, dir.path()), "only once");
    assert!(!fit_ship_args(&mut edited, dir.path()), "edited args stay");
}

#[test]
fn an_untouched_export_step_gets_the_origin_once_its_script_reads_it() {
    let def = for_template(ProjectTemplate::FloatingDiorama);
    let at = def
        .steps
        .iter()
        .position(|s| s.id == "export")
        .expect("export");
    assert_eq!(
        def.steps[at].args[def.steps[at].args.len() - 2..],
        ["--origin".to_string(), "{exportOrigin}".to_string()]
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let mut old = def.clone();
    let n = old.steps[at].args.len();
    old.steps[at].args.truncate(n - 2);
    let mut edited = old.clone();
    edited.steps[at].args.push("--verbose".into());

    std::fs::create_dir_all(dir.path().join("tools")).expect("tools");
    std::fs::write(dir.path().join("tools/export_web.py"), "# exports\n").expect("old");
    assert!(
        !fit_export_origin(&mut old, dir.path()),
        "an older script refuses the flag"
    );
    std::fs::write(
        dir.path().join("tools/export_web.py"),
        include_str!("../../../starter/export_web.py"),
    )
    .expect("new");
    assert!(fit_export_origin(&mut old, dir.path()));
    assert_eq!(old.steps[at].args, def.steps[at].args);
    assert!(!fit_export_origin(&mut old, dir.path()), "only once");
    assert!(
        !fit_export_origin(&mut edited, dir.path()),
        "hand edited args stay"
    );
}
