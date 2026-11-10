use super::*;
use crate::pipeline::graph::blender_headless;
use crate::pipeline::validate;

#[test]
fn render_jobs_are_headless_and_put_a_blend_before_the_script() {
    let glb = render_def(Path::new("/m/crate.glb"), "a.png");
    let args = &glb.steps[0].args;
    assert_eq!(args[0], "-b");
    assert!(blender_headless(args));
    assert!(!args.iter().any(|a| a.ends_with(".blend")));
    assert!(validate(&glb).iter().all(|i| !i.is_error()));

    let blend = render_def(Path::new("/m/scene.blend"), "b.png");
    let args = &blend.steps[0].args;
    assert!(blender_headless(args));
    let open = args
        .iter()
        .position(|a| a == "/m/scene.blend")
        .expect("blend");
    let script = args.iter().position(|a| a == "--python").expect("script");
    assert!(open < script);
    assert_eq!(blend.steps[0].outputs, vec!["b.png".to_string()]);
}

#[test]
fn the_key_follows_the_content_not_the_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("a.glb");
    let b = dir.path().join("b.glb");
    fs::write(&a, b"same").expect("a");
    fs::write(&b, b"same").expect("b");
    let thumbs = dir.path().join("thumbs");
    fs::create_dir_all(&thumbs).expect("thumbs");
    assert_eq!(
        png_for(&thumbs, &a).expect("a"),
        png_for(&thumbs, &b).expect("b")
    );
    fs::write(&b, b"other").expect("b2");
    assert_ne!(
        png_for(&thumbs, &a).expect("a"),
        png_for(&thumbs, &b).expect("b")
    );
}

#[test]
fn store_takes_only_pngs_for_model_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let model = dir.path().join("crate.glb");
    fs::write(&model, b"glb").expect("model");
    let thumbs = dir.path().join("thumbs");
    let mut png = PNG_MAGIC.to_vec();
    png.extend_from_slice(b"rest");
    let saved = store(&thumbs, &model, &png).expect("store");
    assert_eq!(fs::read(&saved).expect("read"), png);
    assert_eq!(
        store(&thumbs, &model, b"GIF89a").expect_err("gif").code(),
        "invalid"
    );
    assert_eq!(
        store(&thumbs, &dir.path().join("notes.txt"), &png)
            .expect_err("txt")
            .code(),
        "invalid"
    );
}

#[test]
fn without_blender_the_preview_stands_in_and_cached_pngs_are_ready() {
    let dir = tempfile::tempdir().expect("tempdir");
    let model = dir.path().join("crate.glb");
    fs::write(&model, b"glb").expect("model");
    let thumbs_dir = dir.path().join("thumbs");
    let service = Thumbnails::new(|_| {});
    let tools = Toolbox::default();
    let first = service
        .request(&thumbs_dir, &model, None, &tools)
        .expect("request");
    assert_eq!(first.state(), ThumbState::Fallback);
    let mut png = PNG_MAGIC.to_vec();
    png.push(0);
    let saved = store(&thumbs_dir, &model, &png).expect("store");
    assert_eq!(
        service
            .request(&thumbs_dir, &model, None, &tools)
            .expect("again"),
        Requested::Ready(saved)
    );
}

#[cfg(unix)]
#[test]
fn a_compressed_model_renders_from_its_raw_export_or_falls_back() {
    use crate::pipeline::tools::ResolvedTool;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::mpsc;

    let dir = tempfile::tempdir().expect("tempdir");
    // A stand-in for Blender that only renders models under a raw folder.
    let fake = dir.path().join("blender");
    fs::write(
        &fake,
        "#!/bin/sh\nwhile [ \"$1\" != \"--model\" ]; do shift; done\nmodel=\"$2\"\nwhile [ \"$1\" != \"--out\" ]; do shift; done\ncase \"$model\" in */raw/*) printf '\\211PNG' > \"$2\";; *) exit 1;; esac\n",
    )
    .expect("fake");
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("chmod");
    let mut tools = Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program: fake,
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    let compressed = |path: &Path, ext: &[&str]| {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        let json = serde_json::json!({"asset": {"version": "2.0"}, "extensionsUsed": ext});
        fs::write(path, serde_json::to_vec(&json).expect("json")).expect("write");
    };
    let shipped = dir.path().join("web/dist/bench.gltf");
    compressed(&shipped, &["KHR_texture_basisu"]);
    let lonely = dir.path().join("elsewhere/lamp.gltf");
    compressed(&lonely, &["EXT_meshopt_compression"]);
    compressed(&dir.path().join("web/raw/bench.gltf"), &[]);

    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let service = Thumbnails::new(move |done| {
        let _ = tx.lock().expect("tx").send(done);
    });
    let thumbs_dir = dir.path().join("thumbs");
    assert_eq!(
        service
            .request(&thumbs_dir, &shipped, None, &tools)
            .expect("shipped"),
        Requested::Rendering
    );
    let done = rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("finished");
    assert_eq!(done.error, None, "rendered from web/raw");
    assert_eq!(done.file, shipped);
    match service
        .request(&thumbs_dir, &lonely, None, &tools)
        .expect("lonely")
    {
        Requested::Fallback(why) => assert!(why.contains("EXT_meshopt_compression")),
        other => panic!("expected the preview to stand in, got {other:?}"),
    }
    service.wait();
}

#[cfg(unix)]
#[test]
fn a_render_reports_every_target_once_it_finishes() {
    use crate::pipeline::tools::ResolvedTool;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::mpsc;

    let dir = tempfile::tempdir().expect("tempdir");
    // A stand-in for Blender: writes the file named after --out.
    let fake = dir.path().join("blender");
    fs::write(
        &fake,
        "#!/bin/sh\nwhile [ \"$1\" != \"--out\" ]; do shift; done\nprintf '\\211PNG' > \"$2\"\n",
    )
    .expect("fake");
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("chmod");
    let mut tools = Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program: fake,
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    let model = dir.path().join("crate.glb");
    fs::write(&model, b"glb").expect("model");
    let thumbs_dir = dir.path().join("thumbs");
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let service = Thumbnails::new(move |done| {
        let _ = tx.lock().expect("tx").send(done);
    });
    let target = |version| ThumbTarget {
        project: "moon-island".into(),
        asset: "crate".into(),
        version,
    };
    let first = service
        .request(&thumbs_dir, &model, Some(target(1)), &tools)
        .expect("first");
    assert_eq!(first, Requested::Rendering);
    let joined = service
        .request(&thumbs_dir, &model, Some(target(2)), &tools)
        .expect("joined");
    assert_eq!(joined, Requested::Rendering);
    let done = rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("finished");
    service.wait();
    assert_eq!(done.error, None);
    let png = done.png.expect("png");
    assert!(png.is_file());
    assert_eq!(done.targets, vec![target(1), target(2)]);
    assert_eq!(done.fingerprint, Fingerprint::of(&model));
    assert_eq!(done.fingerprint.map(|f| f.bytes), Some(3.0));
    assert!(thumbs_dir.join(SCRIPT_FILE).is_file());
    assert_eq!(
        service
            .request(&thumbs_dir, &model, None, &tools)
            .expect("cached"),
        Requested::Ready(png)
    );
}

#[test]
fn a_fingerprint_follows_the_size_and_the_modified_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let model = dir.path().join("crate.glb");
    assert_eq!(Fingerprint::of(&model), None);
    fs::write(&model, b"one").expect("one");
    let first = Fingerprint::of(&model).expect("first");
    assert_eq!(first.bytes, 3.0);
    fs::write(&model, b"longer").expect("longer");
    let second = Fingerprint::of(&model).expect("second");
    assert_ne!(first, second);
    assert_eq!(second.bytes, 6.0);
    assert_eq!(
        Fingerprint::new(7, 5_999_999),
        Fingerprint::new(7, 5_000_000)
    );
    assert_eq!(Fingerprint::new(7, 6_000_000).modified_ms, 6.0);
}

#[test]
fn the_page_light_has_a_cache_name_of_its_own_and_the_studio_keeps_its() {
    let hash = "0123456789abcdef0123456789abcdef0123";
    assert_eq!(
        png_name(hash, ThumbLook::Studio, "rig"),
        "0123456789abcdef0123456789abcdef.png"
    );
    let page = png_name(hash, ThumbLook::Page, "rig one");
    assert!(page.starts_with("0123456789abcdef0123456789abcdef-page-"));
    assert!(page.ends_with(".png"));
    // A changed rig renders the page light again, and only the page light.
    assert_ne!(page, png_name(hash, ThumbLook::Page, "rig two"));
    assert_eq!(
        png_name(hash, ThumbLook::Studio, "rig one"),
        png_name(hash, ThumbLook::Studio, "rig two")
    );
    // The same rig checked out with other line ends is the same rig.
    assert_eq!(
        rig_key("sun = 3\r\nfill = 0.18\r\n"),
        rig_key("sun = 3\nfill = 0.18\n")
    );
    assert_eq!(rig_key(RIG_SCRIPT).len(), 8);
}

#[test]
fn the_page_look_tells_the_script_and_the_studio_job_stays_as_it_was() {
    let model = Path::new("/m/crate.glb");
    let studio = render_def_with(model, "a.png", ThumbLook::Studio.args());
    assert_eq!(
        studio.steps[0].args,
        render_def(model, "a.png").steps[0].args
    );
    assert!(!studio.steps[0].args.iter().any(|a| a == "--look"));
    let page = render_def_with(model, "a.png", ThumbLook::Page.args());
    let args = &page.steps[0].args;
    let at = args.iter().position(|a| a == "--look").expect("look");
    assert_eq!(args[at + 1], "page");
    assert!(blender_headless(args));
}

#[test]
fn only_a_built_asset_gets_the_page_light() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(look_of(dir.path(), "laptop"), ThumbLook::Studio);
    let built = serde_json::json!({
        "version": 1,
        "assets": { "laptop": { "script": "built/laptop.py", "helpers": [], "root": null, "last": null } },
    });
    let file = dir.path().join(crate::built::BUILT_FILE);
    fs::write(&file, serde_json::to_vec(&built).expect("json")).expect("built.json");
    assert_eq!(look_of(dir.path(), "laptop"), ThumbLook::Page);
    assert_eq!(look_of(dir.path(), "desk-lamp"), ThumbLook::Studio);
    // A file that cannot be read lights nothing differently.
    fs::write(&file, b"{ not json").expect("broken");
    assert_eq!(look_of(dir.path(), "laptop"), ThumbLook::Studio);
}

/// What `name = ...` assigns in a script, without a trailing comment.
fn assigned<'a>(text: &'a str, name: &str) -> &'a str {
    let start = format!("{name} = ");
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(&start))
        .unwrap_or_else(|| panic!("no {name} in the script"));
    line[start.len()..]
        .split('#')
        .next()
        .expect("value")
        .trim()
        .trim_end_matches(';')
}

fn number(text: &str, name: &str) -> f64 {
    assigned(text, name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} is not a number"))
}

#[test]
fn the_rig_holds_the_bakes_light_and_the_pages_display() {
    // The light is Build scene's, which Bake light bakes into a model.
    let build = include_str!("../../starter/build_scene.py");
    assert_eq!(
        number(RIG_SCRIPT, "SUN_STRENGTH"),
        number(build, "sun.data.energy")
    );
    assert_eq!(
        number(RIG_SCRIPT, "WORLD_GREY"),
        number(build, "WORLD_GREY")
    );
    let turn: Vec<f64> = assigned(build, "sun.rotation_euler")
        .trim_matches(|c| c == '(' || c == ')')
        .split(',')
        .map(|v| v.trim().parse().expect("a turn"))
        .collect();
    assert_eq!(number(RIG_SCRIPT, "SUN_TILT"), turn[0]);
    // The same sun, mirrored from the front right to the front left.
    assert_eq!(number(RIG_SCRIPT, "SUN_TURN"), -turn[2]);
    assert_eq!(turn[1], 0.0);

    // The display is the reference page's renderer.
    let site =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/site/src/scene/Workbench.tsx");
    let page = fs::read_to_string(&site).expect("the site's renderer settings");
    assert_eq!(
        assigned(&page, "gl.toneMapping"),
        "THREE.ACESFilmicToneMapping"
    );
    assert_eq!(assigned(RIG_SCRIPT, "TONE"), "\"aces-filmic\"");
    assert_eq!(
        number(RIG_SCRIPT, "EXPOSURE"),
        number(&page, "gl.toneMappingExposure")
    );
}

#[cfg(unix)]
#[test]
fn a_page_render_takes_the_rig_along_and_never_stands_in_for_a_studio_one() {
    use crate::pipeline::tools::ResolvedTool;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::mpsc;

    let dir = tempfile::tempdir().expect("tempdir");
    // A stand-in for Blender: notes what it was told and writes the picture.
    let fake = dir.path().join("blender");
    fs::write(
        &fake,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$(dirname \"$0\")/told.txt\"\nwhile [ \"$1\" != \"--out\" ]; do shift; done\nprintf '\\211PNG' > \"$2\"\n",
    )
    .expect("fake");
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("chmod");
    let mut tools = Toolbox {
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
        ..Toolbox::default()
    };
    tools.tools.insert(
        ToolId::Blender,
        ResolvedTool {
            program: fake,
            prefix: Vec::new(),
            version: "test".into(),
        },
    );
    let model = dir.path().join("laptop.glb");
    fs::write(&model, b"glb").expect("model");
    let thumbs_dir = dir.path().join("thumbs");
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let service = Thumbnails::new(move |done| {
        let _ = tx.lock().expect("tx").send(done);
    });
    let wait = || {
        rx.recv_timeout(std::time::Duration::from_secs(20))
            .expect("finished")
    };

    let asked = service
        .request_look(&thumbs_dir, &model, None, &tools, ThumbLook::Page)
        .expect("page");
    assert_eq!(asked, Requested::Rendering);
    let page = wait().png.expect("the page picture");
    assert_eq!(
        page,
        png_for_look(&thumbs_dir, &model, ThumbLook::Page).expect("name")
    );
    assert_eq!(
        fs::read_to_string(thumbs_dir.join(RIG_FILE)).expect("the rig"),
        RIG_SCRIPT
    );
    let told = fs::read_to_string(dir.path().join("told.txt")).expect("told");
    assert!(told.contains("--look\npage\n"), "{told}");

    // The studio picture of the same model is another file, rendered apart.
    let studio = service
        .request(&thumbs_dir, &model, None, &tools)
        .expect("studio");
    assert_eq!(studio, Requested::Rendering);
    let plain = wait().png.expect("the studio picture");
    assert_eq!(plain, png_for(&thumbs_dir, &model).expect("name"));
    assert_ne!(plain, page);
    service.wait();
    // Both are cached now, each under its own name.
    for look in [ThumbLook::Page, ThumbLook::Studio] {
        let again = service
            .request_look(&thumbs_dir, &model, None, &tools, look)
            .expect("cached");
        assert_eq!(again.state(), ThumbState::Ready, "{look:?}");
    }
}
