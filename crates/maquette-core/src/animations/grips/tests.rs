use std::fs;
use std::path::PathBuf;

use super::*;
use crate::recipes::grips::{GRIP_VERSION, GripHands, GripSide};
use crate::recipes::{
    KeyEase, LayerKind, LayerMode, LayerSettings, PoseKey, RecipeBlank, RecipeChange, RecipeFile,
    RecipeLayer, empty_pose_layer,
};
use crate::workspace::WorkspaceConfig;

/// A workspace with the project Frostveil and its recipe `hold`, on the
/// rest pose, whose pose keys turn both hands, a finger and the chest.
struct Ws {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    config: WorkspaceConfig,
    frost: String,
}

const RECIPE: &str = "hold";

impl Ws {
    fn dir(&self) -> PathBuf {
        crate::project::dir(&self.root, &self.config.layout, &self.frost).expect("dir")
    }

    fn manifest(&self) -> crate::project::ProjectManifest {
        crate::project::load(&self.root, &self.config.layout, &self.frost).expect("manifest")
    }

    fn recipe(&self) -> RecipeFile {
        recipes_get(self).recipe
    }

    fn version(&self) -> String {
        recipes_get(self).version
    }

    fn change(&self, change: RecipeChange) {
        crate::recipes::update(
            &self.dir(),
            &self.manifest(),
            RECIPE,
            RecipeChange {
                expected: Some(self.version()),
                ..change
            },
        )
        .expect("change");
    }

    fn apply(&self, request: ApplyGrip) -> CoreResult<GripApplied> {
        apply(
            &self.root,
            &self.config.layout,
            &ApplyGrip {
                project: self.frost.clone(),
                recipe: RECIPE.into(),
                ..request
            },
        )
    }
}

fn recipes_get(w: &Ws) -> crate::recipes::RecipeRead {
    crate::recipes::get(&w.dir(), &w.manifest(), RECIPE).expect("recipe")
}

fn key(frame: u32, bones: &[(&str, [f32; 3])]) -> PoseKey {
    PoseKey {
        clip: 0,
        frame,
        ease: KeyEase::Smooth,
        pose: bones.iter().map(|(b, t)| ((*b).to_string(), *t)).collect(),
    }
}

fn ws() -> Ws {
    let tmp = tempfile::tempdir().expect("temp");
    let root = tmp.path().to_path_buf();
    let config = WorkspaceConfig::default();
    let frost = crate::project::create(
        &root,
        &config.layout,
        crate::project::NewProject {
            name: "Frostveil".into(),
            template: crate::project::ProjectTemplate::FloatingDiorama,
            target: crate::game::ProjectTarget::Website,
            game: None,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project")
    .id;
    let w = Ws {
        _tmp: tmp,
        root,
        config,
        frost,
    };
    crate::animations::recipes::create(&w.root, &w.dir(), None, Some("Hold")).expect("recipe");
    let mut keys = empty_pose_layer("l-1", LayerMode::Replace);
    keys.keys = vec![
        key(
            10,
            &[
                ("chest", [0.0, -18.0, 0.0]),
                ("rightHand", [4.0, 0.0, -8.0]),
                ("rightIndexProximal", [0.0, 0.0, 62.0]),
                ("leftHand", [0.0, 6.0, 0.0]),
            ],
        ),
        key(40, &[("rightIndexProximal", [0.0, 0.0, 20.0])]),
    ];
    w.change(RecipeChange {
        layers: Some(vec![keys]),
        ..RecipeChange::default()
    });
    w
}

fn pose(bones: &[(&str, [f32; 3])]) -> BonePose {
    bones.iter().map(|(b, t)| ((*b).to_string(), *t)).collect()
}

fn longsword() -> GripFile {
    GripFile {
        version: GRIP_VERSION,
        name: "Longsword, two hands".into(),
        revision: 1,
        weapon: "longsword".into(),
        hands: GripHolds {
            holds: GripSide::Right,
            supports: Some(GripSide::Left),
        },
        pose: pose(&[
            ("rightHand", [4.0, 0.0, -8.0]),
            ("rightIndexProximal", [0.0, 0.0, 62.0]),
            ("leftHand", [0.0, 6.0, 0.0]),
        ]),
    }
}

/// Saves `grip` as a new library grip, or as the next revision of `entry`.
fn saved(w: &Ws, grip: &GripFile, entry: Option<&str>) -> CoreResult<GripSaved> {
    let plan = plan_save(&w.root, grip, entry)?;
    save_grip(
        &w.root,
        &GripSave {
            grip: grip.clone(),
            entry: plan.entry.clone(),
            expected: plan.entry_version,
            source: None,
        },
    )
}

#[test]
fn a_grip_saves_as_a_new_library_grip_and_reads_back() {
    let w = ws();
    assert!(list(&w.root).expect("list").is_empty(), "no folder yet");
    let plan = plan_save(&w.root, &longsword(), None).expect("plan");
    assert_eq!((plan.entry.as_deref(), plan.revision), (None, 1));
    assert_eq!(plan.file, "library/grips/longsword-two-hands/r1.grip.json");
    assert!(plan.bytes > 100 && !plan.same);
    assert_eq!(plan.changes, ["A new grip for both hands: 3 bones."]);
    let source = GripSource {
        project: w.frost.clone(),
        recipe: RECIPE.into(),
        layer: "l-1".into(),
        frame: 10,
    };
    let done = save_grip(
        &w.root,
        &GripSave {
            grip: longsword(),
            entry: None,
            expected: None,
            source: Some(source),
        },
    )
    .expect("save");
    assert_eq!(
        (done.entry.as_str(), done.name.as_str(), done.revision),
        ("longsword-two-hands", "Longsword, two hands", 1)
    );

    let (entry, version) = load_entry(&w.root, &done.entry).expect("entry");
    assert_eq!(version, done.entry_version);
    assert!(crate::recipes::is_entry_uid(&entry.uid), "{}", entry.uid);
    let rev = &entry.revisions[0];
    assert_eq!(rev.file, "r1.grip.json");
    assert_eq!(
        (rev.project.as_deref(), rev.layer.as_deref(), rev.frame),
        (Some(w.frost.as_str()), Some("l-1"), Some(10))
    );
    let folder = entry_dir(&w.root, &done.entry).expect("dir");
    let text = fs::read_to_string(folder.join("r1.grip.json")).expect("r1");
    assert_eq!(u32::try_from(text.len()).expect("len"), plan.bytes);
    for word in ["keys", "frames", "mesh", "uid", "project"] {
        assert!(!text.contains(word), "{word} in {text}");
    }

    let listed = list(&w.root).expect("list");
    assert_eq!(listed.len(), 1);
    let line = &listed[0];
    assert_eq!((line.latest, line.weapon.as_str()), (1, "longsword"));
    assert_eq!(
        (line.group.as_str(), line.hands_words.as_str()),
        ("swords", "both hands")
    );
    assert_eq!(line.pose, longsword().pose);
    assert_eq!(line.uid, entry.uid);
    let doc = get(&w.root, &done.entry, None).expect("get");
    assert_eq!((doc.revision, &doc.grip), (1, &longsword()));
    assert!(matches!(
        get(&w.root, &done.entry, Some(2)),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        get(&w.root, "no-such-grip", None),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn each_save_adds_a_revision_says_what_changed_and_never_overwrites() {
    let w = ws();
    let first = saved(&w, &longsword(), None).expect("r1");
    let id = first.entry.as_str();
    let same = plan_save(&w.root, &longsword(), Some(id)).expect("plan");
    assert!(same.same && same.changes.is_empty());
    let nothing = saved(&w, &longsword(), Some(id)).expect_err("nothing new");
    assert!(nothing.to_string().contains("nothing to save"), "{nothing}");

    let mut tighter = longsword();
    tighter.name = "Another name".into();
    tighter.weapon = "rapier".into();
    tighter
        .pose
        .insert("rightIndexProximal".into(), [0.0, 0.0, 75.0]);
    tighter
        .pose
        .insert("rightThumbDistal".into(), [10.0, 0.0, 0.0]);
    tighter.pose.remove("leftHand");
    tighter.hands.supports = None;
    let plan = plan_save(&w.root, &tighter, Some(id)).expect("plan");
    assert_eq!((plan.latest, plan.revision), (Some(1), 2));
    assert_eq!(plan.file, "library/grips/longsword-two-hands/r2.grip.json");
    assert_eq!(
        plan.grip.name, "Longsword, two hands",
        "a revision keeps the grip's name"
    );
    assert_eq!(
        plan.changes,
        [
            "Holds rapier instead of longsword.",
            "Poses right hand, where it posed both hands.",
            "Poses 1 bone more.",
            "Leaves 1 bone out.",
            "Turns 1 bone another way.",
        ]
    );

    // A revision needs the version its plan read, and that version to hold.
    let bare = GripSave {
        grip: tighter.clone(),
        entry: Some(id.into()),
        expected: None,
        source: None,
    };
    let refused = save_grip(&w.root, &bare).expect_err("no version");
    assert!(
        refused.to_string().contains("needs the version"),
        "{refused}"
    );
    let stale = GripSave {
        expected: Some("0000000000000000".into()),
        ..bare.clone()
    };
    assert!(matches!(
        save_grip(&w.root, &stale),
        Err(CoreError::Moved(_))
    ));
    let r1 = entry_dir(&w.root, id).expect("dir").join("r1.grip.json");
    let before = fs::read_to_string(&r1).expect("r1");
    let second = saved(&w, &tighter, Some(id)).expect("r2");
    assert_eq!(second.revision, 2);
    assert_eq!(
        fs::read_to_string(&r1).expect("r1"),
        before,
        "r1 is never rewritten"
    );
    let doc = get(&w.root, id, None).expect("newest");
    assert_eq!(
        (doc.revision, doc.grip.name.as_str()),
        (2, "Longsword, two hands")
    );
    assert_eq!(get(&w.root, id, Some(1)).expect("r1").grip, longsword());
    let line = &list(&w.root).expect("list")[0];
    assert_eq!((line.latest, line.hands_words.as_str()), (2, "right hand"));
    assert_eq!(line.revisions.len(), 2);
}

#[test]
fn a_taken_name_takes_a_number_and_a_weapon_has_to_be_one() {
    let w = ws();
    let first = saved(&w, &longsword(), None).expect("first");
    let plan = plan_save(&w.root, &longsword(), None).expect("plan");
    assert_eq!(plan.entry_name, "Longsword, two hands (2)");
    assert_eq!(
        plan.file,
        "library/grips/longsword-two-hands-2/r1.grip.json"
    );
    let second = saved(&w, &longsword(), None).expect("second");
    assert_eq!(second.entry, "longsword-two-hands-2");
    assert_eq!(second.name, "Longsword, two hands (2)");
    let (a, _) = load_entry(&w.root, &first.entry).expect("a");
    let (b, _) = load_entry(&w.root, &second.entry).expect("b");
    assert_ne!(a.uid, b.uid, "each grip has its own uid");

    let odd = GripFile {
        weapon: "lightsaber".into(),
        ..longsword()
    };
    for refused in [
        plan_save(&w.root, &odd, None).map(|_| ()),
        saved(&w, &odd, None).map(|_| ()),
    ] {
        let err = refused.expect_err("unknown weapon");
        assert!(
            err.to_string().contains("not a weapon a grip holds"),
            "{err}"
        );
    }
    let mut chest = longsword();
    chest.pose.insert("chest".into(), [0.0, 5.0, 0.0]);
    assert!(matches!(
        saved(&w, &chest, None),
        Err(CoreError::Invalid(_))
    ));
    assert_eq!(
        list(&w.root).expect("list").len(),
        2,
        "nothing half written"
    );
}

#[test]
fn removing_moves_files_aside_never_reuses_a_number_and_breaks_old_links() {
    let w = ws();
    let first = saved(&w, &longsword(), None).expect("r1");
    let id = first.entry.as_str();
    let only = remove_revision(&w.root, id, 1, None).expect_err("the only one");
    assert!(only.to_string().contains("only revision"), "{only}");
    let mut next = longsword();
    next.pose.insert("rightHand".into(), [9.0, 0.0, 0.0]);
    saved(&w, &next, Some(id)).expect("r2");
    assert!(matches!(
        remove_revision(&w.root, id, 2, Some("0000000000000000")),
        Err(CoreError::Moved(_))
    ));
    assert!(matches!(
        remove_revision(&w.root, id, 7, None),
        Err(CoreError::NotFound(_))
    ));
    let entry = remove_revision(&w.root, id, 2, None).expect("remove r2");
    assert_eq!((entry.revisions.len(), entry.last_revision), (1, 2));
    assert!(
        !entry_dir(&w.root, id)
            .expect("dir")
            .join("r2.grip.json")
            .exists()
    );
    let aside: Vec<String> = fs::read_dir(dir(&w.root).join(REMOVED_DIR))
        .expect("removed")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        aside.iter().any(|n| n.ends_with("r2.grip.json")),
        "{aside:?}"
    );
    assert_eq!(
        saved(&w, &next, Some(id)).expect("again").revision,
        3,
        "r2 is never reused"
    );

    // A link to the grip holds until the grip goes; an id taken again is
    // another grip, so the old link reads as gone.
    let (was, _) = load_entry(&w.root, id).expect("entry");
    let link = link_to(&was, 1);
    assert!(linked_entry(&w.root, &link).expect("linked").is_some());
    let went = remove_entry(&w.root, id, None).expect("remove");
    assert!(went.starts_with(".removed/longsword-two-hands-"), "{went}");
    assert!(
        list(&w.root).expect("list").is_empty(),
        "the removed folder never lists"
    );
    assert!(linked_entry(&w.root, &link).expect("gone").is_none());
    let again = saved(&w, &longsword(), None).expect("same id again");
    assert_eq!(again.entry, id);
    assert!(
        linked_entry(&w.root, &link).expect("another").is_none(),
        "another uid"
    );
    let by_id = crate::recipes::GripFrom { uid: None, ..link };
    assert!(
        linked_entry(&w.root, &by_id).expect("by id").is_some(),
        "no uid goes by id"
    );
}

#[test]
fn a_grip_lays_a_copy_with_a_link_and_takes_a_newer_revision() {
    let w = ws();
    let first = saved(&w, &longsword(), None).expect("r1");
    let id = first.entry.as_str();
    let before = recipes_get(&w);
    let laid = w
        .apply(ApplyGrip {
            grip: id.into(),
            hands: Some(GripHands::Right),
            expected: Some(before.version.clone()),
            start: Some(0.25),
            end: Some(0.75),
            ..ApplyGrip::default()
        })
        .expect("apply");
    assert_eq!((laid.layer.as_str(), laid.updated), ("l-2", false));
    assert_eq!(
        (laid.revision, laid.latest, laid.name.as_str()),
        (1, 1, "Longsword, two hands")
    );
    let layer = &laid.doc.recipe.layers[1];
    assert_eq!(
        (layer.kind, layer.bone.as_str()),
        (LayerKind::Grip, "rightHand")
    );
    assert_eq!(layer.mode, Some(LayerMode::Replace));
    assert_eq!(
        layer.pose.as_ref().map(BonePose::len),
        Some(2),
        "the right hand's bones"
    );
    assert_eq!((layer.settings.start, layer.settings.end), (0.25, 0.75));
    let (entry, _) = load_entry(&w.root, id).expect("entry");
    assert_eq!(layer.from, Some(link_to(&entry, 1)));
    assert_ne!(
        laid.doc.motion_version, before.motion_version,
        "a grip moves the model"
    );
    let now = links(&w.root, &laid.doc.recipe);
    assert_eq!(now.len(), 1);
    assert_eq!(
        (now[0].latest, now[0].newer, now[0].gone),
        (Some(1), false, false)
    );

    // r2 turns only the left hand another way: the library offers it, and
    // taking it moves the link but not what this recipe's right hand does.
    let mut left = longsword();
    left.pose.insert("leftHand".into(), [0.0, 30.0, 0.0]);
    saved(&w, &left, Some(id)).expect("r2");
    let offered = links(&w.root, &w.recipe());
    assert_eq!((offered[0].latest, offered[0].newer), (Some(2), true));
    let motion = recipes_get(&w).motion_version;
    let relinked = w
        .apply(ApplyGrip {
            grip: id.into(),
            layer: Some("l-2".into()),
            ..ApplyGrip::default()
        })
        .expect("update");
    assert!(relinked.updated);
    assert_eq!(relinked.revision, 2);
    assert_eq!(
        relinked.doc.motion_version, motion,
        "a link and its revision stay out"
    );
    let layer = &relinked.doc.recipe.layers[1];
    assert_eq!(layer.from.as_ref().map(|f| f.revision), Some(2));
    assert_eq!(
        (layer.settings.start, layer.settings.end),
        (0.25, 0.75),
        "where it sat"
    );

    // r3 curls the right index: taking it changes the motion.
    let mut curled = left.clone();
    curled
        .pose
        .insert("rightIndexProximal".into(), [0.0, 0.0, 80.0]);
    saved(&w, &curled, Some(id)).expect("r3");
    let taken = w
        .apply(ApplyGrip {
            grip: id.into(),
            layer: Some("l-2".into()),
            ..ApplyGrip::default()
        })
        .expect("take r3");
    assert_ne!(taken.doc.motion_version, motion, "the grip pose counts");
    let pose = taken.doc.recipe.layers[1].pose.as_ref().expect("pose");
    assert_eq!(pose["rightIndexProximal"], [0.0, 0.0, 80.0]);
    // An older revision can be taken by number, and other hands asked for.
    let back = w
        .apply(ApplyGrip {
            grip: id.into(),
            revision: Some(1),
            hands: Some(GripHands::Both),
            layer: Some("l-2".into()),
            ..ApplyGrip::default()
        })
        .expect("back to r1");
    assert_eq!((back.revision, back.latest), (1, 3));
    assert_eq!(
        back.doc.recipe.layers[1].pose.as_ref().map(BonePose::len),
        Some(3)
    );

    remove_entry(&w.root, id, None).expect("remove");
    let gone = links(&w.root, &w.recipe());
    assert_eq!(
        (gone[0].gone, gone[0].latest, gone[0].newer),
        (true, None, false)
    );
    assert_eq!(
        w.recipe().layers[1].pose.as_ref().map(BonePose::len),
        Some(3),
        "the copy stays"
    );
}

#[test]
fn laying_a_grip_refuses_what_the_recipe_would() {
    let w = ws();
    let id = saved(&w, &longsword(), None).expect("r1").entry;
    let with = |request: ApplyGrip| {
        w.apply(ApplyGrip {
            grip: id.clone(),
            ..request
        })
    };
    assert!(matches!(
        w.apply(ApplyGrip {
            grip: "no-such-grip".into(),
            ..ApplyGrip::default()
        }),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        w.apply(ApplyGrip {
            grip: "../recipes".into(),
            ..ApplyGrip::default()
        }),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        with(ApplyGrip {
            revision: Some(4),
            ..ApplyGrip::default()
        }),
        Err(CoreError::NotFound(_))
    ));
    let pose_keys = with(ApplyGrip {
        layer: Some("l-1".into()),
        ..ApplyGrip::default()
    })
    .expect_err("pose keys are not a grip");
    assert!(
        pose_keys.to_string().contains("not a grip change"),
        "{pose_keys}"
    );
    assert!(matches!(
        with(ApplyGrip {
            expected: Some("0000000000000000".into()),
            ..ApplyGrip::default()
        }),
        Err(CoreError::Moved(_))
    ));
    let aimless = with(ApplyGrip {
        target: Some("hilt".into()),
        ..ApplyGrip::default()
    })
    .expect_err("no such blank");
    assert!(aimless.to_string().contains("the blank hilt"), "{aimless}");
    let backwards = with(ApplyGrip {
        start: Some(0.8),
        end: Some(0.2),
        ..ApplyGrip::default()
    });
    assert!(matches!(backwards, Err(CoreError::Invalid(_))));
    assert_eq!(w.recipe().layers.len(), 1, "nothing was written");

    // A blank it holds, a named change, and the recipe's cap of 32 changes.
    w.change(RecipeChange {
        blanks: Some(vec![RecipeBlank {
            id: "hilt".into(),
            label: "The sword she holds".into(),
            kind: crate::recipes::BlankKind::Point,
            prop: None,
            point: Some([0.0, 1.0, 0.3]),
        }]),
        ..RecipeChange::default()
    });
    let held = with(ApplyGrip {
        target: Some("hilt".into()),
        layer: Some("grip-a".into()),
        weight: Some(0.5),
        ease_in: Some(0),
        mode: Some(LayerMode::Add),
        ..ApplyGrip::default()
    })
    .expect("held");
    let layer = &held.doc.recipe.layers[1];
    assert_eq!(
        (held.layer.as_str(), layer.id.as_str()),
        ("grip-a", "grip-a")
    );
    assert_eq!(layer.settings.target.as_deref(), Some("hilt"));
    assert_eq!(
        (layer.settings.weight, layer.settings.ease_in),
        (0.5, Some(0))
    );
    assert_eq!(layer.mode, Some(LayerMode::Add));
    let filler = RecipeLayer::new(
        "",
        LayerKind::Nudge,
        "spine",
        LayerSettings {
            rotation: Some([1.0, 0.0, 0.0]),
            ..LayerSettings::default()
        },
    );
    let mut full = w.recipe().layers;
    full.extend(std::iter::repeat_n(filler, 30));
    w.change(RecipeChange {
        layers: Some(full),
        ..RecipeChange::default()
    });
    let over = with(ApplyGrip::default()).expect_err("a 33rd change");
    assert!(over.to_string().contains("at most 32 changes"), "{over}");
}

#[test]
fn a_grip_is_read_from_the_pose_keys_on_disk() {
    let w = ws();
    let ask = |request: GripFromKeys| {
        from_keys(
            &w.root,
            &w.config.layout,
            &GripFromKeys {
                project: w.frost.clone(),
                recipe: RECIPE.into(),
                name: "Hilt".into(),
                weapon: "Longsword".into(),
                ..request
            },
        )
    };
    let keyed = ask(GripFromKeys {
        layer: "l-1".into(),
        ..GripFromKeys::default()
    })
    .expect("keyed");
    assert_eq!(keyed.source.frame, 10, "the first key on a hand");
    assert_eq!(keyed.recipe_name, "Hold");
    assert_eq!(keyed.grip.pose.len(), 3, "the chest stays the recipe's");
    assert_eq!(keyed.grip.weapon, "longsword");
    assert_eq!(keyed.grip.hands.supports, Some(GripSide::Left));
    assert!(
        keyed.notes.is_empty(),
        "replace keys read as a grip plays: {:?}",
        keyed.notes
    );

    // Halfway to the second key the finger is halfway there; one hand only.
    let later = ask(GripFromKeys {
        layer: "l-1".into(),
        frame: Some(25),
        hands: Some(GripHands::Right),
        ..GripFromKeys::default()
    })
    .expect("later");
    assert_eq!(later.grip.pose["rightIndexProximal"], [0.0, 0.0, 41.0]);
    assert_eq!(
        (later.grip.pose.len(), later.grip.hands.supports),
        (2, None)
    );
    let far = ask(GripFromKeys {
        layer: "l-1".into(),
        frame: Some(9_000),
        ..GripFromKeys::default()
    });
    assert_eq!(far.expect("kept within the bake").source.frame, 60);

    let mut added = w.recipe().layers;
    added[0].mode = Some(LayerMode::Add);
    w.change(RecipeChange {
        layers: Some(added),
        ..RecipeChange::default()
    });
    let noted = ask(GripFromKeys {
        layer: "l-1".into(),
        ..GripFromKeys::default()
    })
    .expect("noted");
    assert!(
        noted.notes[0].contains("add to the clip"),
        "{:?}",
        noted.notes
    );

    assert!(matches!(
        ask(GripFromKeys {
            layer: "l-9".into(),
            ..GripFromKeys::default()
        }),
        Err(CoreError::NotFound(_))
    ));
    let chest = ask(GripFromKeys {
        layer: "l-1".into(),
        bones: Some(vec!["chest".into()]),
        ..GripFromKeys::default()
    })
    .expect_err("no hand bone");
    assert!(
        chest
            .to_string()
            .contains("turn no hand, finger or arm bone"),
        "{chest}"
    );
    let unknown = from_keys(
        &w.root,
        &w.config.layout,
        &GripFromKeys {
            project: w.frost.clone(),
            recipe: RECIPE.into(),
            layer: "l-1".into(),
            name: "Hilt".into(),
            weapon: "lightsaber".into(),
            ..GripFromKeys::default()
        },
    )
    .expect_err("unknown weapon");
    assert!(unknown.to_string().contains("not a weapon"), "{unknown}");
}

#[test]
fn a_broken_grip_lists_with_its_problem_and_can_still_be_removed() {
    let w = ws();
    let id = saved(&w, &longsword(), None).expect("r1").entry;
    let folder = entry_dir(&w.root, &id).expect("dir");
    fs::write(folder.join("r1.grip.json"), "{ not json").expect("break r1");
    let listed = list(&w.root).expect("list");
    assert_eq!((listed.len(), listed[0].latest), (1, 0));
    assert!(listed[0].problem.is_some() && !listed[0].version.is_empty());
    assert_eq!(
        listed[0].name, "Longsword, two hands",
        "the entry still reads"
    );

    fs::write(folder.join(ENTRY_FILE), "{ not json").expect("break the entry");
    let listed = list(&w.root).expect("list");
    assert_eq!(listed[0].name, id, "only its folder is known");
    assert!(listed[0].problem.is_some() && listed[0].version.is_empty());
    remove_entry(&w.root, &id, None).expect("a broken grip still goes");
    assert!(list(&w.root).expect("list").is_empty());
    assert!(matches!(
        remove_entry(&w.root, &id, None),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn an_entry_keeps_at_most_500_revisions_and_names_only_revision_files() {
    let w = ws();
    let id = saved(&w, &longsword(), None).expect("r1").entry;
    let folder = entry_dir(&w.root, &id).expect("dir");
    let (mut entry, _) = load_entry(&w.root, &id).expect("entry");
    let one = entry.revisions[0].clone();
    entry.revisions = (1..=u32::try_from(MAX_REVISIONS).expect("cap"))
        .map(|n| LibraryGripRevision {
            revision: n,
            file: revision_file(n),
            ..one.clone()
        })
        .collect();
    entry.last_revision = 500;
    write_json(&folder.join(ENTRY_FILE), &entry).expect("write");
    let mut newest = longsword();
    newest.revision = 500;
    fs::write(
        folder.join("r500.grip.json"),
        crate::recipes::grips::text(&newest).expect("text"),
    )
    .expect("r500");
    let mut next = longsword();
    next.pose.insert("rightHand".into(), [9.0, 0.0, 0.0]);
    let full = saved(&w, &next, Some(&id)).expect_err("a 501st revision");
    assert!(
        full.to_string().contains("more than 500 revisions"),
        "{full}"
    );
    assert!(
        !folder.join("r501.grip.json").exists(),
        "nothing was written"
    );

    entry.revisions.truncate(1);
    entry.revisions[0].file = "../../../maquette.json".into();
    write_json(&folder.join(ENTRY_FILE), &entry).expect("write");
    let outside = load_entry(&w.root, &id).expect_err("not a revision file");
    assert!(
        outside.to_string().contains("not a revision file"),
        "{outside}"
    );
}
