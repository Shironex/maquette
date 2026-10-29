use std::collections::BTreeMap;

use super::*;
use crate::workspace::AvatarDefaults;
use maquette_glb::vrm::{CommercialUsage, LICENSE_URL};

fn hero() -> AvatarSpec {
    let mut spec = AvatarDefaults {
        author: " Test Author ".to_string(),
        commercial_usage: CommercialUsage::PersonalProfit,
        ..AvatarDefaults::default()
    }
    .spec("Hero");
    spec.bones
        .insert("leftEye".to_string(), Some("Eye_L".to_string()));
    spec.bones.insert("jaw".to_string(), None);
    spec
}

fn message(result: CoreResult<SceneAvatars>) -> String {
    match result {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn defaults_fill_a_new_spec_at_the_vrm_defaults() {
    let spec = AvatarDefaults::default().spec(" Hero ");
    assert_eq!(spec.meta.name, "Hero");
    assert!(spec.meta.authors.is_empty());
    assert_eq!(spec.meta.license_url, LICENSE_URL);
    assert!(!spec.meta.allow_redistribution);
    assert_eq!(
        spec.meta,
        VrmMeta {
            name: "Hero".to_string(),
            ..VrmMeta::default()
        }
    );
    assert_eq!(spec.material, MaterialMode::MtoonSoft);
    // A draft without authors saves; writing it is what asks for them.
    spec.validate().expect("draft");
    assert!(spec.meta.validate().is_err());

    let hero = hero();
    assert_eq!(hero.meta.authors, ["Test Author"]);
    assert_eq!(hero.meta.commercial_usage, CommercialUsage::PersonalProfit);
    hero.meta.validate().expect("complete");
}

#[test]
fn a_missing_file_reads_as_no_avatars_and_a_saved_one_round_trips() {
    let dir = tempfile::tempdir().expect("dir");
    assert_eq!(load(dir.path()).expect("load"), SceneAvatars::default());

    let written = set(dir.path(), "hero", Some(hero())).expect("set");
    assert_eq!(load(dir.path()).expect("load"), written);
    let spec = &written.avatars["hero"];
    assert_eq!(spec.bones.get("jaw"), Some(&None));

    let text = std::fs::read_to_string(path(dir.path())).expect("read");
    let json: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(json["avatars"]["hero"]["meta"]["licenseUrl"], LICENSE_URL);
    assert_eq!(json["avatars"]["hero"]["material"], "mtoon-soft");

    // Another asset's spec stays when one is set or removed.
    set(
        dir.path(),
        "villain",
        Some(AvatarDefaults::default().spec("Villain")),
    )
    .expect("set");
    let after = set(dir.path(), "villain", None).expect("remove");
    assert_eq!(after.avatars.keys().collect::<Vec<_>>(), ["hero"]);
}

#[test]
fn saving_trims_the_text_and_drops_empty_entries() {
    let mut spec = hero();
    spec.meta.references = vec![" https://example.com/ref ".to_string(), "  ".to_string()];
    spec.meta.contact_information = Some("   ".to_string());
    let stored = normalized(&SceneAvatars {
        version: 0,
        avatars: BTreeMap::from([("hero".to_string(), spec)]),
    });
    assert_eq!(stored.version, AVATAR_VERSION);
    let meta = &stored.avatars["hero"].meta;
    assert_eq!(meta.references, ["https://example.com/ref"]);
    assert_eq!(meta.contact_information, None);
}

#[test]
fn bad_specs_are_refused_with_the_asset_named() {
    let dir = tempfile::tempdir().expect("dir");
    let mut wing = hero();
    wing.bones
        .insert("leftWing".to_string(), Some("Wing_L".to_string()));
    assert!(
        message(set(dir.path(), "hero", Some(wing)))
            .starts_with("hero: leftWing is not a VRM humanoid bone.")
    );

    let mut blank = hero();
    blank
        .bones
        .insert("head".to_string(), Some(" ".to_string()));
    assert!(message(set(dir.path(), "hero", Some(blank))).contains("a joint name must have"));

    let mut licence = hero();
    licence.meta.license_url = "https://example.com/licence".to_string();
    assert!(message(set(dir.path(), "hero", Some(licence))).contains("The licence URL must be"));

    let mut author = hero();
    author.meta.authors.push(String::new());
    assert!(message(set(dir.path(), "hero", Some(author))).contains("An author name is blank."));

    // Nothing was written.
    assert!(!path(dir.path()).exists());
}

#[test]
fn a_newer_or_broken_file_is_never_saved_over() {
    let dir = tempfile::tempdir().expect("dir");
    std::fs::write(path(dir.path()), r#"{ "version": 99, "avatars": {} }"#).expect("write");
    assert!(matches!(
        load(dir.path()),
        Err(CoreError::UnsupportedVersion { found: 99, .. })
    ));
    assert!(set(dir.path(), "hero", Some(hero())).is_err());

    std::fs::write(path(dir.path()), "{ not json").expect("write");
    assert!(
        message(set(dir.path(), "hero", Some(hero()))).starts_with("avatar.json could not be read")
    );
    assert_eq!(
        std::fs::read_to_string(path(dir.path())).expect("read"),
        "{ not json"
    );
}
