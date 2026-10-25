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
