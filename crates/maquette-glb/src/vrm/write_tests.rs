use std::path::Path;

use serde_json::Value;

use super::*;
use crate::fixture::{self, HumanoidSpec};

/// The golden JSON chunk of the written fixture. Set `MAQUETTE_BLESS=1` to
/// write it again after a deliberate change, then run the formatter.
const GOLDEN: &str = "src/vrm/golden/humanoid_vrm.json";

fn spec() -> WriteSpec {
    WriteSpec {
        meta: VrmMeta {
            name: "Hero".to_string(),
            authors: vec!["Test Author".to_string()],
            ..VrmMeta::default()
        },
        ..WriteSpec::default()
    }
}

fn written(model: &HumanoidSpec, spec: &WriteSpec) -> Written {
    write(&fixture::humanoid(model), spec, Path::new(".")).expect("write")
}

/// The JSON chunk, read back with serde.
fn chunk(bytes: &[u8]) -> Value {
    container::split(bytes).expect("split").0
}

fn error(bytes: &[u8], spec: &WriteSpec) -> String {
    match write(bytes, spec, Path::new(".")) {
        Ok(_) => panic!("expected the writer to refuse"),
        Err(e) => e.to_string(),
    }
}

fn node_index(json: &Value, name: &str) -> Option<u64> {
    json["nodes"]
        .as_array()?
        .iter()
        .position(|n| n["name"] == name)
        .and_then(|i| u64::try_from(i).ok())
}

#[test]
fn the_written_fixture_matches_the_golden_json_and_parses_again() {
    let model = HumanoidSpec {
        scale_noise: true,
        ..HumanoidSpec::default()
    };
    let mut spec = spec();
    spec.thumbnail = Some(Thumbnail {
        bytes: b"\x89PNG\r\n\x1a\nnot really a png".to_vec(),
        mime: "image/png".to_string(),
    });
    let out = written(&model, &spec);
    assert!(out.unresolved.is_empty(), "{:?}", out.unresolved);

    // The gltf crate reads it with validation on.
    let gltf = gltf::Gltf::from_slice(&out.bytes).expect("valid glTF");
    assert_eq!(gltf.document.animations().count(), 0);
    let blob = gltf.blob.as_deref().expect("bin");
    let image = gltf.document.images().last().expect("thumbnail");
    let gltf::image::Source::View { view, mime_type } = image.source() else {
        panic!("thumbnail in a buffer view");
    };
    assert_eq!(mime_type, "image/png");
    let thumb = spec.thumbnail.as_ref().map(|t| t.bytes.as_slice());
    assert_eq!(
        blob.get(view.offset()..view.offset() + view.length()),
        thumb
    );
    assert!(blob.len() >= view.offset() + view.length());

    let json = chunk(&out.bytes);
    let vrm = &json["extensions"][VRMC_VRM];
    assert_eq!(vrm["specVersion"], "1.0");
    assert_eq!(vrm["meta"]["name"], "Hero");
    assert_eq!(vrm["meta"]["licenseUrl"], LICENSE_URL);
    assert_eq!(vrm["meta"]["avatarPermission"], "onlyAuthor");
    assert_eq!(vrm["meta"]["commercialUsage"], "personalNonProfit");
    assert_eq!(vrm["meta"]["thumbnailImage"], image.index());
    let bones = vrm["humanoid"]["humanBones"].as_object().expect("bones");
    assert_eq!(bones.len(), 52);
    assert_eq!(
        bones["hips"]["node"].as_u64(),
        node_index(&json, "mixamorig:Hips")
    );
    assert_eq!(vrm["lookAt"]["type"], "expression");
    let used: Vec<&str> = json["extensionsUsed"]
        .as_array()
        .expect("used")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(used.contains(&VRMC_VRM) && used.contains(&VRMC_MATERIALS_MTOON));
    assert_eq!(
        json["nodes"][1]["scale"],
        serde_json::json!([1.0, 1.0, 1.0])
    );

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    if std::env::var_os("MAQUETTE_BLESS").is_some() {
        let pretty = serde_json::to_string_pretty(&json).expect("json") + "\n";
        std::fs::write(&path, pretty).expect("bless");
    }
    // Compared as JSON, so the formatter may lay the file out its own way.
    let golden: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("golden file")).expect("json");
    assert_eq!(json, golden, "the written JSON moved from {GOLDEN}");
}

#[test]
fn the_written_file_reads_back_with_the_same_rig() {
    let out = written(&HumanoidSpec::default(), &spec());
    let again = readiness(&out.bytes, Path::new("."), &BoneOverrides::new()).expect("readiness");
    assert!(again.ready);
    assert_eq!(again.mapped, 52);
    // The clip is gone, so the note about dropping it is too.
    assert!(!again.findings.iter().any(|f| f.id == "animations"));
}

#[test]
fn pbr_mode_leaves_the_materials_as_they_are() {
    let model = HumanoidSpec::default();
    let mut spec = spec();
    spec.material = MaterialMode::Pbr;
    let before = chunk(&fixture::humanoid(&model));
    let after = chunk(&written(&model, &spec).bytes);
    assert_eq!(before["materials"], after["materials"]);
    assert!(
        !after["extensionsUsed"]
            .as_array()
            .expect("used")
            .iter()
            .any(|v| v == VRMC_MATERIALS_MTOON)
    );
}

#[test]
fn mtoon_soft_shades_from_the_base_color() {
    let json = chunk(&written(&HumanoidSpec::default(), &spec()).bytes);
    for material in json["materials"].as_array().expect("materials") {
        let mtoon = &material["extensions"][VRMC_MATERIALS_MTOON];
        assert_eq!(mtoon["specVersion"], "1.0");
        assert_eq!(mtoon["outlineWidthMode"], "none");
        let base = &material["pbrMetallicRoughness"];
        if let Some(texture) = base.get("baseColorTexture") {
            assert_eq!(mtoon["shadeMultiplyTexture"]["index"], texture["index"]);
        }
        assert!(base.get("baseColorFactor").is_none() || mtoon["shadeColorFactor"].is_array());
    }
}

#[test]
fn pose_blockers_are_written_and_handed_back() {
    let model = HumanoidSpec {
        yaw: 90.0,
        arm_drop: 45.0,
        lift: 0.1,
        ..HumanoidSpec::default()
    };
    let out = written(&model, &spec());
    let ids: Vec<&str> = out.unresolved.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, ["facing", "arms", "ground"]);
    assert_eq!(out.unresolved[0].level, FindingLevel::Blocker);
    gltf::Gltf::from_slice(&out.bytes).expect("valid glTF");
}

#[test]
fn the_writer_refuses_what_vrm_cannot_hold() {
    let humanoid = fixture::humanoid(&HumanoidSpec::default());

    assert!(error(b"not a model", &spec()).contains("binary glTF"));
    let unrigged = error(&fixture::clean_prop(), &spec());
    assert!(unrigged.starts_with("This model cannot be written as VRM yet."));
    assert!(unrigged.contains("No skinned mesh"));

    let mut nameless = spec();
    nameless.meta.name = " ".to_string();
    assert_eq!(error(&humanoid, &nameless), "The avatar needs a name.");
    let mut no_author = spec();
    no_author.meta.authors.clear();
    assert_eq!(error(&humanoid, &no_author), "Name at least one author.");
    let mut licence = spec();
    licence.meta.license_url = "https://example.com/".to_string();
    assert!(error(&humanoid, &licence).starts_with("The licence URL must be"));

    let mut unknown = spec();
    unknown
        .overrides
        .insert("leftWing".to_string(), Some("mixamorig:Hips".to_string()));
    assert!(error(&humanoid, &unknown).starts_with("leftWing is not a VRM humanoid bone."));

    let mut gif = spec();
    gif.thumbnail = Some(Thumbnail {
        bytes: vec![0; 4],
        mime: "image/gif".to_string(),
    });
    assert_eq!(
        error(&humanoid, &gif),
        "A VRM thumbnail is a PNG or JPEG image."
    );

    let mut no_hips = spec();
    no_hips.overrides.insert("hips".to_string(), None);
    assert!(error(&humanoid, &no_hips).contains("No joint maps to hips"));
}

#[test]
fn the_container_splits_and_joins_without_loss() {
    let bytes = fixture::humanoid(&HumanoidSpec::default());
    let (json, bin) = container::split(&bytes).expect("split");
    let again = container::join(&json, bin.clone()).expect("join");
    let (json2, bin2) = container::split(&again).expect("split again");
    assert_eq!(json, json2);
    assert_eq!(bin, bin2);
    gltf::Gltf::from_slice(&again).expect("valid glTF");
}

/// Writes a real model from disk as VRM, for a local look. Set
/// `MAQUETTE_VRM_PROBE` to a `.glb`, and `MAQUETTE_VRM_OUT` to keep the
/// `.vrm`, and run with `--ignored --nocapture`.
#[test]
#[ignore = "reads a model from MAQUETTE_VRM_PROBE"]
fn probe_write_a_model_from_disk() {
    let Ok(file) = std::env::var("MAQUETTE_VRM_PROBE") else {
        return;
    };
    let bytes = std::fs::read(&file).expect("read");
    let out = write(&bytes, &spec(), Path::new(".")).expect("write");
    gltf::Gltf::from_slice(&out.bytes).expect("valid glTF");
    println!("{} bytes in, {} bytes out", bytes.len(), out.bytes.len());
    for f in &out.unresolved {
        println!("[{:?}] {}", f.level, f.title);
    }
    if let Ok(path) = std::env::var("MAQUETTE_VRM_OUT") {
        std::fs::write(path, &out.bytes).expect("write out");
    }
}
