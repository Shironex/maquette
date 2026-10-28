use super::bones::{self, MapChoice, PairMatch};
use super::feet;
use super::review::{self, ClipRun, ReviewStatus, RunReport};
use super::settings::{self, ClipSetting};
use super::uses::{self, AssetAnimations, BakedClip};
use crate::animations::tests::{MIXAMO, library_clip, mixamo_bones};

/// A model rig with its own names, Rigify style, and a skirt bone.
fn model_bones() -> Vec<String> {
    [
        "hips",
        "spine",
        "spine_01",
        "chest",
        "neck",
        "head",
        "shoulder.L",
        "upper_arm.L",
        "forearm.L",
        "hand.L",
        "shoulder.R",
        "upper_arm.R",
        "forearm.R",
        "hand.R",
        "thigh.L",
        "shin.L",
        "foot.L",
        "toe.L",
        "thigh.R",
        "shin.R",
        "foot.R",
        "toe.R",
        "skirt_front_01",
    ]
    .map(String::from)
    .to_vec()
}

const TRIPO_RIG: [&str; 22] = [
    "Hip",
    "Waist",
    "Spine01",
    "Spine02",
    "NeckTwist01",
    "Head",
    "L_Clavicle",
    "L_Upperarm",
    "L_Forearm",
    "L_Hand",
    "R_Clavicle",
    "R_Upperarm",
    "R_Forearm",
    "R_Hand",
    "L_Thigh",
    "L_Calf",
    "L_Foot",
    "L_ToeBase",
    "R_Thigh",
    "R_Calf",
    "R_Foot",
    "R_ToeBase",
];

fn pair<'a>(pairs: &'a [bones::BonePair], bone: &str) -> &'a bones::BonePair {
    pairs.iter().find(|p| p.bone == bone).expect("bone")
}

#[test]
fn a_mixamo_clip_maps_onto_a_model_by_its_names() {
    let pairs = bones::suggest(&mixamo_bones(), &model_bones(), &[], &[]);
    assert_eq!(pairs.len(), MIXAMO.len());
    let hips = pair(&pairs, "Hips");
    assert_eq!(hips.source.as_deref(), Some("mixamorig:Hips"));
    assert_eq!(hips.target.as_deref(), Some("hips"));
    assert_eq!(hips.how, PairMatch::ByName);
    let arm = pair(&pairs, "LeftArm");
    assert_eq!(arm.target.as_deref(), Some("upper_arm.L"));
    // Read the Unreal way, spine_01 is the first spine bone, so the chest
    // is taken one bone early and the upper chest is left for a look.
    assert_eq!(pair(&pairs, "Spine").target.as_deref(), Some("spine_01"));
    assert_eq!(pair(&pairs, "Spine2").how, PairMatch::Missing);
    assert_eq!(pairs.iter().filter(|p| p.target.is_none()).count(), 1);
    assert!(bones::needs_look(&pairs, false));
    let mapped = bones::pairs_for(&mixamo_bones(), &pairs);
    assert!(mapped.contains(&("mixamorig:RightLeg".into(), "shin.R".into())));
    assert!(bones::keep_rest(&pairs).contains(&"chest".to_string()));
    assert!(!bones::keep_rest(&pairs).contains(&"upper_arm.L".to_string()));
}

#[test]
fn a_tripo_rig_with_mixamo_names_maps_exactly() {
    for prefix in ["mixamorig:", "mixamorig_"] {
        let tripo: Vec<String> = MIXAMO.iter().map(|b| format!("{prefix}{b}")).collect();
        let pairs = bones::suggest(&mixamo_bones(), &tripo, &[], &[]);
        assert!(
            pairs.iter().all(|p| p.how == PairMatch::Exact),
            "{prefix}: {pairs:?}"
        );
        assert!(!bones::needs_look(&pairs, false));
        let mapped = bones::pairs_for(&mixamo_bones(), &pairs);
        assert!(mapped.contains(&("mixamorig:Spine2".into(), format!("{prefix}Spine2"))));
    }
}

#[test]
fn unverified_tripo_names_still_fill_the_humanoid_bones() {
    let tripo: Vec<String> = TRIPO_RIG.map(String::from).to_vec();
    let pairs = bones::suggest(&tripo, &model_bones(), &[], &[]);
    assert!(pairs.iter().all(|p| p.source.is_some()), "{pairs:?}");
    assert_eq!(pair(&pairs, "Spine").source.as_deref(), Some("Waist"));
    assert_eq!(pair(&pairs, "Neck").source.as_deref(), Some("NeckTwist01"));
    assert_eq!(pair(&pairs, "LeftUpLeg").source.as_deref(), Some("L_Thigh"));
    // A Tripo model takes a Mixamo clip the same way.
    let onto = bones::suggest(&mixamo_bones(), &tripo, &[], &[]);
    assert_eq!(pair(&onto, "Spine1").target.as_deref(), Some("Spine01"));
    assert_eq!(
        pair(&onto, "RightForeArm").target.as_deref(),
        Some("R_Forearm")
    );
}

#[test]
fn a_missing_bone_asks_for_a_look_until_the_map_is_saved_or_picked() {
    let mut model = model_bones();
    model.retain(|b| b != "neck");
    let pairs = bones::suggest(&mixamo_bones(), &model, &[], &[]);
    assert_eq!(pair(&pairs, "Neck").how, PairMatch::Missing);
    assert!(bones::needs_look(&pairs, false));
    assert!(
        !bones::needs_look(&pairs, true),
        "a saved map is never forced open"
    );

    let picked = [MapChoice {
        bone: "Neck".into(),
        target: Some("chest".into()),
    }];
    let pairs = bones::suggest(&mixamo_bones(), &model, &[], &picked);
    assert_eq!(pair(&pairs, "Neck").how, PairMatch::Picked);
    let saved = bones::choices(&pairs);
    let again = bones::suggest(&mixamo_bones(), &model, &saved, &[]);
    assert_eq!(pair(&again, "Neck").target.as_deref(), Some("chest"));
    assert_eq!(pair(&again, "Neck").how, PairMatch::Saved);
    // A choice naming a bone the model lacks is dropped.
    let stale = [MapChoice {
        bone: "Head".into(),
        target: Some("helmet".into()),
    }];
    let pairs = bones::suggest(&mixamo_bones(), &model, &stale, &[]);
    assert_eq!(pair(&pairs, "Head").target.as_deref(), Some("head"));
}

#[test]
fn names_drop_the_pack_words_and_settings_are_made_safe() {
    let pack = "Pro Sword and Shield Pack";
    assert_eq!(
        settings::default_name("Sword And Shield Idle", pack),
        "sword_idle"
    );
    assert_eq!(
        settings::default_name("Standing Dodge Left", "Singles"),
        "dodge_left"
    );
    assert_eq!(settings::default_name("Pro Pack", "Pro Pack"), "pro_pack");

    let idle = library_clip("sword-idle", "Sword And Shield Idle", pack, "a");
    let block = library_clip("sword-block", "Sword And Shield Block", pack, "b");
    let clips = [idle.clone(), block.clone()];
    let wild = ClipSetting {
        name: "Sword Idle!".into(),
        start: 30,
        end: 10,
        speed: 9.0,
        ..settings::default_for(&idle)
    };
    let blank = ClipSetting {
        name: "  ".into(),
        start: 4,
        end: 400,
        speed: f32::NAN,
        ..settings::default_for(&block)
    };
    let taken = ["sword_idle".to_string()];
    let out = settings::normalise(&[wild, blank], &clips, &taken).expect("settings");
    assert_eq!(out[0].name, "sword_idle_2");
    assert_eq!(
        (out[0].start, out[0].end),
        (0, 37),
        "a trim that keeps nothing keeps it all"
    );
    assert!((out[0].speed - settings::SPEED_MAX).abs() < f32::EPSILON);
    assert_eq!(out[1].name, "sword_block");
    assert_eq!((out[1].start, out[1].end), (4, 37));
    assert!((out[1].speed - 1.0).abs() < f32::EPSILON);

    let twice = [settings::default_for(&idle), settings::default_for(&idle)];
    assert!(settings::normalise(&twice, &clips, &[]).is_err());
    let unknown = ClipSetting {
        clip: "nope".into(),
        ..settings::default_for(&idle)
    };
    assert!(settings::normalise(&[unknown], &clips, &[]).is_err());
}

fn run(id: &str) -> ClipRun {
    ClipRun {
        id: id.into(),
        name: id.replace('-', "_"),
        frames: 30,
        travel: 0.87,
        farthest: 0.9,
        foot: "left".into(),
        ..ClipRun::default()
    }
}

#[test]
fn the_review_flags_sliding_feet_and_fails_keys_in_the_wrong_units() {
    let report = RunReport {
        clips: vec![
            run("dodge-left"),
            ClipRun {
                slide_cm: 4.2,
                slide_from: 9,
                slide_to: 14,
                ..run("sword-run")
            },
            ClipRun {
                farthest: 44.0,
                ..run("crouch-guard")
            },
            ClipRun {
                error: Some("the clip has no keys".into()),
                ..run("broken")
            },
        ],
        seconds: 52.0,
        driven: 22,
        aligned_deg: 47.0,
        hips_scale: 0.94,
    };
    let review = review::review(&report, |id| id == "dodge-left", vec!["Neck".into()]);
    let status: Vec<ReviewStatus> = review.clips.iter().map(|c| c.status).collect();
    assert_eq!(
        status,
        [
            ReviewStatus::Good,
            ReviewStatus::Look,
            ReviewStatus::Failed,
            ReviewStatus::Failed
        ]
    );
    assert_eq!(review.clips[0].note, "Looks right, 0.87 m root motion");
    assert_eq!(
        review.clips[1].note,
        "The left foot slides 4.2 cm while planted, frames 9 to 14"
    );
    assert_eq!(review.clips[1].slide_frames, Some([9, 14]));
    assert_eq!(review.clips[1].travel, None, "it plays in place");
    assert!(review.clips[2].note.contains("44 m"));
    assert_eq!(review.baked().count(), 2);
    assert_eq!(review.at_rest, ["Neck"]);
}

#[test]
fn a_fixed_library_clip_offers_the_models_that_baked_it_an_update() {
    let mut idle = library_clip("sword-idle", "Sword And Shield Idle", "Pack", "a");
    let block = library_clip("sword-block", "Sword And Shield Block", "Pack", "b");
    let baked = |clip: &crate::animations::AnimationClip, version| BakedClip {
        setting: settings::default_for(clip),
        revision: clip.revision,
        library_name: clip.name.clone(),
        version,
    };
    let mut entry = AssetAnimations::default();
    let review = review::review(&RunReport::default(), |_| false, Vec::new());
    uses::record_bake(
        &mut entry,
        vec![baked(&idle, 4), baked(&block, 4)],
        review.clone(),
        4,
    );
    assert!(uses::outdated(&entry, &[idle.clone(), block.clone()]).is_empty());

    idle.revision = 2;
    let outdated = uses::outdated(&entry, &[idle.clone(), block.clone()]);
    assert_eq!(outdated.len(), 1);
    assert_eq!(outdated[0].clip, "sword-idle");

    // Baking it again replaces the older entry.
    uses::record_bake(&mut entry, vec![baked(&idle, 5)], review, 5);
    assert_eq!(entry.baked.len(), 2);
    assert!(uses::outdated(&entry, &[idle, block]).is_empty());
    assert_eq!(entry.review_version, Some(5));
}

#[test]
fn the_project_record_round_trips_and_refuses_a_newer_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(uses::load(dir.path()).expect("empty").assets.len(), 0);
    let choice = MapChoice {
        bone: "Neck".into(),
        target: Some("neck_01".into()),
    };
    uses::update(dir.path(), "elsheria", |e| e.map = vec![choice.clone()]).expect("write");
    let entry = uses::of_asset(dir.path(), "elsheria").expect("read");
    assert_eq!(entry.map, [choice]);
    std::fs::write(
        dir.path().join(uses::USES_FILE),
        r#"{"version": 99, "assets": {}}"#,
    )
    .expect("newer");
    assert!(uses::load(dir.path()).is_err());
}

/// A foot track along X: `(x, z)` per frame, metres.
fn track(points: &[(f32, f32)]) -> feet::Track {
    points.iter().map(|&(x, z)| [x, 0.0, z]).collect()
}

#[test]
fn a_foot_is_planted_while_it_is_low_and_slow() {
    // Down and still for five frames, lifted, then down again sliding.
    let walk = track(&[
        (0.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.01),
        (0.0, 0.0),
        (0.0, 0.0),
        (0.2, 0.15),
        (0.4, 0.2),
        (0.6, 0.0),
        (0.62, 0.0),
        (0.64, 0.0),
        (0.66, 0.02),
    ]);
    // The frame it lands on still comes in fast, so the second plant starts after it.
    assert_eq!(feet::planted_runs(&walk), vec![[0, 4], [8, 10]]);
    // Scraping the ground fast is a swing, not a plant.
    let scrape: Vec<(f32, f32)> = (0..6u8).map(|i| (f32::from(i) * 0.12, 0.0)).collect();
    assert!(feet::planted_runs(&track(&scrape)).is_empty());
    // Two frames down is a touch, not a plant.
    let touch = track(&[(0.0, 0.3), (0.0, 0.0), (0.0, 0.0), (0.0, 0.3)]);
    assert!(feet::planted_runs(&touch).is_empty());
}

#[test]
fn the_slide_is_the_drift_from_where_the_foot_went_down() {
    let still = track(&[(0.3, 0.0); 4]);
    let sliding = track(&[
        (0.0, 0.0),
        (0.01, 0.0),
        (0.03, 0.0),
        (0.02, 0.0),
        (0.0, 0.3),
    ]);
    let feet_now: std::collections::BTreeMap<String, feet::Track> =
        [("left".into(), still), ("right".into(), sliding)].into();
    let slide = feet::slide(&feet_now);
    assert_eq!((slide.foot.as_str(), slide.from, slide.to), ("right", 0, 3));
    assert!((slide.cm - 3.0).abs() < 0.01, "{slide:?}");
    let pins = feet::pins(&feet_now);
    assert_eq!(pins["left"], vec![[0, 3]]);
    assert_eq!(pins["right"], vec![[0, 3]]);
}

#[test]
fn a_pinned_clip_is_reviewed_against_its_slide_before() {
    let sliding = track(&[(0.0, 0.0), (0.01, 0.0), (0.04, 0.0), (0.0, 0.3)]);
    let held = track(&[(0.0, 0.0), (0.001, 0.0), (0.002, 0.0), (0.0, 0.3)]);
    let mut first = ClipRun {
        feet: [("left".to_string(), sliding)].into(),
        ..run("foot-slide")
    };
    first.measure();
    assert!(
        (first.slide_cm - 4.0).abs() < 0.01,
        "measured from the track"
    );
    let mut pinned = ClipRun {
        feet: [("left".to_string(), held)].into(),
        unpinned_cm: Some(first.slide_cm),
        ..run("foot-slide")
    };
    pinned.measure();
    let report = RunReport {
        clips: vec![first, pinned.clone()],
        ..RunReport::default()
    };
    let review = review::review(&report, |_| false, Vec::new());
    assert_eq!(review.clips[0].status, ReviewStatus::Look);
    assert_eq!(review.clips[1].status, ReviewStatus::Good);
    assert_eq!(
        review.clips[1].note,
        "Feet pinned, the slide is down from 4.0 cm to 0.2 cm"
    );
    assert_eq!(review.clips[1].unpinned_cm, Some(4.0));

    // Still sliding after the pin stays to look at, with both numbers.
    let mut stuck = ClipRun {
        slide_cm: 2.5,
        slide_from: 3,
        slide_to: 9,
        ..pinned
    };
    stuck.feet.clear();
    let report = RunReport {
        clips: vec![stuck],
        ..RunReport::default()
    };
    let review = review::review(&report, |_| false, Vec::new());
    assert_eq!(review.clips[0].status, ReviewStatus::Look);
    assert!(
        review.clips[0]
            .note
            .contains("still slides 2.5 cm (was 4.0 cm)"),
        "{}",
        review.clips[0].note
    );
    // A leg that is not mapped says why its feet stay as they were.
    let unmapped = ClipRun {
        slide_cm: 3.0,
        foot: "left".into(),
        pin_error: Some("the left leg is not mapped".into()),
        ..run("foot-slide")
    };
    let report = RunReport {
        clips: vec![unmapped],
        ..RunReport::default()
    };
    let note = &review::review(&report, |_| false, Vec::new()).clips[0].note;
    assert!(
        note.ends_with("could not be pinned: the left leg is not mapped"),
        "{note}"
    );
}
