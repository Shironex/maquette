use std::fs;
use std::path::{Path, PathBuf};

use super::*;

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
    trash: PathBuf,
}

fn project() -> Project {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("moon-island");
    fs::create_dir_all(&root).expect("project");
    let trash = dir.path().join("trash").join("moon-island");
    Project {
        _dir: dir,
        root,
        trash,
    }
}

fn note(id: &str, x: i32, y: i32) -> PlanNode {
    PlanNode {
        id: id.into(),
        kind: NodeKind::Note,
        x,
        y,
        width: None,
        title: String::new(),
        text: "Frost veil slows time".into(),
        image: None,
        url: None,
        asset: None,
        direction: None,
        by: None,
        person_wrote: false,
        stop: None,
    }
}

fn edge(id: &str, from: &str, to: &str) -> PlanEdge {
    PlanEdge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        from_side: Some(Side::Right),
        to_side: Some(Side::Left),
        kind: EdgeKind::Relates,
        label: None,
        by: None,
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("read")
}

#[test]
fn a_new_canvas_gets_a_file_named_after_it_and_a_free_id() {
    let p = project();
    let first = create(&p.root, "Combat ideas", PlanPreset::Brainstorm).expect("create");
    assert_eq!(first.id, "combat-ideas");
    assert_eq!(first.file, "plans/combat-ideas.plan.json");
    assert!(p.root.join("plans/combat-ideas.plan.json").is_file());
    let second = create(&p.root, "Combat ideas", PlanPreset::Brainstorm).expect("again");
    assert_eq!(second.id, "combat-ideas-2");
    let odd = create(&p.root, "\u{96ea}", PlanPreset::Brainstorm).expect("japanese");
    assert_eq!(odd.id, "canvas");
    let reserved = create(&p.root, "Con", PlanPreset::Brainstorm).expect("reserved");
    assert_eq!(reserved.id, "con-1");
    assert!(create(&p.root, "  ", PlanPreset::Brainstorm).is_err());
    assert!(create(&p.root, "two\nlines", PlanPreset::Brainstorm).is_err());
}

#[test]
fn a_save_writes_sorted_json_and_the_same_canvas_writes_the_same_bytes() {
    let p = project();
    let made = create(&p.root, "Story", PlanPreset::Story).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![note("n-b", 10, 20), note("n-a", -3, 7)];
    plan.edges = vec![edge("e-2", "n-b", "n-a"), edge("e-1", "n-a", "n-b")];
    let saved = save(&p.root, &made.id, &plan).expect("save");
    let ids: Vec<&str> = saved.plan.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["n-a", "n-b"]);
    assert_eq!(saved.plan.edges[0].id, "e-1");
    assert_eq!((saved.plan.nodes[1].x, saved.plan.nodes[1].y), (10, 20));
    assert_eq!(saved.plan.preset, PlanPreset::Story);

    let file = p.root.join(&saved.file);
    let before = read(&file);
    // The same canvas in another order changes nothing, not even `edited`.
    plan.nodes.reverse();
    let again = save(&p.root, &made.id, &plan).expect("again");
    assert_eq!(again.plan.edited, saved.plan.edited);
    assert_eq!(read(&file), before);
    assert!(before.ends_with('\n'));
    assert!(before.contains("\"fromSide\": \"right\""));
    assert!(before.contains("\"kind\": \"relates\""));
}

#[test]
fn a_canvas_round_trips_and_a_newer_file_says_so() {
    let p = project();
    let made = create(&p.root, "Ideas", PlanPreset::Brainstorm).expect("create");
    let back = get(&p.root, &made.id).expect("get");
    assert_eq!(back.plan, made.plan);

    let file = p.root.join(&made.file);
    let text = read(&file).replace("\"version\": 2", "\"version\": 9");
    fs::write(&file, text).expect("newer");
    match get(&p.root, &made.id) {
        Err(CoreError::UnsupportedVersion { found, supported }) => {
            assert_eq!((found, supported), (9, PLAN_VERSION));
        }
        other => panic!("expected a version error, got {other:?}"),
    }
}

#[test]
fn validation_refuses_what_the_file_cannot_hold() {
    let base = PlanFile::new("Ideas", PlanPreset::Brainstorm);
    let with = |nodes: Vec<PlanNode>, edges: Vec<PlanEdge>| PlanFile {
        nodes,
        edges,
        ..base.clone()
    };
    assert!(validate(&with(vec![note("a", 0, 0)], vec![])).is_ok());
    assert!(validate(&with(vec![note("a", 0, 0), note("a", 1, 1)], vec![])).is_err());
    assert!(validate(&with(vec![note("a b", 0, 0)], vec![])).is_err());
    assert!(validate(&with(vec![note("a", i32::MIN, 0)], vec![])).is_err());
    assert!(validate(&with(vec![note("a", 2_000_000, 0)], vec![])).is_err());
    assert!(validate(&with(vec![note("a", 0, 0)], vec![edge("e", "a", "ghost")])).is_err());
    let narrow = PlanNode {
        width: Some(10),
        ..note("a", 0, 0)
    };
    assert!(validate(&with(vec![narrow], vec![])).is_err());

    let picture = |path: &str| PlanNode {
        kind: NodeKind::Image,
        image: Some(path.into()),
        ..note("p", 0, 0)
    };
    assert!(validate(&with(vec![picture("refs/porch.png")], vec![])).is_ok());
    for bad in [
        "/etc/passwd",
        "../x.png",
        "C:/x.png",
        "refs\\x.png",
        "a//b.png",
    ] {
        assert!(
            validate(&with(vec![picture(bad)], vec![])).is_err(),
            "{bad}"
        );
    }

    let link = |url: &str| PlanNode {
        kind: NodeKind::Link,
        url: Some(url.into()),
        ..note("l", 0, 0)
    };
    assert!(validate(&with(vec![link("https://example.com/a")], vec![])).is_ok());
    assert!(validate(&with(vec![link("http://localhost:4321")], vec![])).is_ok());
    for bad in [
        "javascript:alert(1)",
        "file:///etc",
        "https://a b",
        "https://",
    ] {
        assert!(validate(&with(vec![link(bad)], vec![])).is_err(), "{bad}");
    }
}

#[test]
fn a_rename_keeps_the_file_and_a_removed_canvas_goes_to_the_trash() {
    let p = project();
    let made = create(&p.root, "Ideas", PlanPreset::Brainstorm).expect("create");
    let renamed = rename(&p.root, &made.id, " Lantern festival ").expect("rename");
    assert_eq!(renamed.name, "Lantern festival");
    assert_eq!(renamed.id, "ideas");
    assert_eq!(
        get(&p.root, "ideas").expect("get").plan.name,
        "Lantern festival"
    );

    let moved = remove(&p.root, &p.trash, "ideas").expect("remove");
    assert!(!p.root.join("plans/ideas.plan.json").exists());
    assert!(Path::new(&moved).is_file());
    assert!(moved.contains("plans"));
    // A canvas removed meanwhile is not written back by a late save.
    assert!(save(&p.root, "ideas", &made.plan).is_err());
    assert!(remove(&p.root, &p.trash, "ideas").is_err());
    assert!(get(&p.root, "../ideas").is_err());
}

#[test]
fn the_list_counts_each_canvas_and_still_shows_a_broken_one() {
    let p = project();
    assert!(list(&p.root).expect("empty").is_empty());
    let made = create(&p.root, "Ideas", PlanPreset::Brainstorm).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = vec![
        note("a", 0, 0),
        PlanNode {
            kind: NodeKind::Link,
            url: Some("https://example.com".into()),
            ..note("b", 0, 0)
        },
    ];
    plan.edges = vec![edge("e", "a", "b")];
    save(&p.root, &made.id, &plan).expect("save");
    fs::write(p.root.join("plans/broken.plan.json"), "{ nope").expect("broken");
    fs::write(p.root.join("plans/notes.txt"), "not a canvas").expect("other");

    let all = list(&p.root).expect("list");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, "broken");
    assert!(all[0].problem.is_some());
    let ideas = &all[1];
    assert_eq!(
        (ideas.nodes, ideas.edges, ideas.links, ideas.images),
        (2, 1, 1, 0)
    );
    assert_eq!(ideas.preset, Some(PlanPreset::Brainstorm));
}

#[test]
fn a_picture_inside_the_project_is_linked_and_one_outside_is_copied_once() {
    let p = project();
    fs::create_dir_all(p.root.join("refs")).expect("refs");
    fs::write(p.root.join("refs/porch.png"), b"png").expect("inside");
    let inside = images::add(&p.root, &p.root.join("refs/porch.png")).expect("inside");
    assert_eq!(inside.path, "refs/porch.png");
    assert!(!inside.copied);

    let outside_dir = p.root.parent().expect("parent").join("elsewhere");
    fs::create_dir_all(&outside_dir).expect("elsewhere");
    let outside = outside_dir.join("Porch at Dusk.PNG");
    fs::write(&outside, b"other png").expect("outside");
    let copied = images::add(&p.root, &outside).expect("copy");
    assert!(copied.copied);
    assert!(copied.path.starts_with("plans/images/porch-at-dusk-"));
    assert!(copied.path.ends_with(".png"));
    assert_eq!(copied.name, "Porch at Dusk");
    assert!(p.root.join(&copied.path).is_file());
    let again = images::add(&p.root, &outside).expect("again");
    assert_eq!(again.path, copied.path);
    let count = fs::read_dir(p.root.join("plans/images"))
        .expect("dir")
        .count();
    assert_eq!(count, 1, "no temp file left and no second copy");

    let svg = outside_dir.join("logo.svg");
    fs::write(&svg, b"<svg/>").expect("svg");
    assert!(images::add(&p.root, &svg).is_err());
    assert!(images::add(&p.root, &outside_dir.join("missing.png")).is_err());

    let mut plan = PlanFile::new("Pictures", PlanPreset::Brainstorm);
    plan.nodes = vec![PlanNode {
        kind: NodeKind::Image,
        image: Some(copied.path.clone()),
        ..note("p", 0, 0)
    }];
    let paths = images::paths_of(&p.root, &plan);
    assert_eq!(
        Path::new(&paths[&copied.path]),
        p.root
            .join("plans")
            .join("images")
            .join(Path::new(&copied.path).file_name().expect("file name"))
    );
}

#[test]
fn picture_paths_and_ids_refuse_what_breaks_on_some_system() {
    for bad in [
        "//host/share/a.png",
        r"\\?\C:\x.png",
        "C:x.png",
        "con",
        "concepts/con.png",
        "concepts/NUL/a.png",
        "concepts/a.png.",
        "concepts/a.png ",
        "concepts /a.png",
        ".../a.png",
        "concepts/../a.png",
        "/abs/a.png",
        "",
    ] {
        assert!(!is_project_relative(bad), "{bad:?} must be refused");
    }
    assert!(is_project_relative("concepts/__pool/veil-gate.webp"));
    assert!(is_project_relative("plans/images/console-1a2b3c4d.png"));
    for bad in ["con", "nul", "com1", "lpt9", "aux", "", "-a", "a-", "A"] {
        assert!(!is_plan_id(bad), "{bad:?} must not be a canvas id");
    }
    assert!(is_plan_id("con-1"));
    assert!(path(Path::new("/p"), "nul").is_err());
}

#[test]
fn each_kind_keeps_only_its_own_fields() {
    let mut plan = PlanFile::new("Kinds", PlanPreset::Brainstorm);
    let picture = |path: &str| PlanNode {
        kind: NodeKind::Image,
        image: Some(path.into()),
        ..note("n-1", 0, 0)
    };
    plan.nodes = vec![picture("concepts/a.PNG")];
    validate(&plan).expect("a picture");
    for (node, why) in [
        (
            picture("concepts/a.svg"),
            "a kind of picture the canvas does not copy",
        ),
        (
            picture(&format!("{}.png", "a/".repeat(600))),
            "a path too long",
        ),
        (
            PlanNode {
                url: Some("https://a.example".into()),
                ..picture("a.png")
            },
            "an address on a picture",
        ),
        (
            PlanNode {
                image: Some("a.png".into()),
                ..note("n-1", 0, 0)
            },
            "a picture on a note",
        ),
        (
            PlanNode {
                url: Some("https://a.example".into()),
                ..note("n-1", 0, 0)
            },
            "an address on a note",
        ),
        (
            PlanNode {
                by: Some("x".repeat(BY_MAX_CHARS + 1)),
                ..note("n-1", 0, 0)
            },
            "a long by",
        ),
        (
            PlanNode {
                by: Some("agent\nname".into()),
                ..note("n-1", 0, 0)
            },
            "a by on two lines",
        ),
    ] {
        plan.nodes = vec![node];
        assert!(validate(&plan).is_err(), "{why} must be refused");
    }
    plan.nodes = vec![note("n-1", 0, 0), note("n-2", 0, 0)];
    plan.edges = vec![PlanEdge {
        by: Some("y".repeat(BY_MAX_CHARS + 1)),
        ..edge("e-1", "n-1", "n-2")
    }];
    assert!(
        validate(&plan).is_err(),
        "an edge's long by must be refused"
    );
}

#[test]
fn an_oversize_file_lists_as_broken_without_being_read() {
    let p = project();
    let made = create(&p.root, "Huge", PlanPreset::Brainstorm).expect("create");
    let file = path(&p.root, &made.id).expect("path");
    let handle = fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .expect("open");
    handle.set_len(MAX_FILE_BYTES + 1).expect("grow");
    drop(handle);
    let listed = list(&p.root).expect("list");
    assert_eq!(listed.len(), 1);
    assert!(
        listed[0]
            .problem
            .as_deref()
            .is_some_and(|m| m.contains("too big"))
    );
    assert!(get(&p.root, &made.id).is_err());
    remove(&p.root, &p.trash, &made.id).expect("a broken canvas can still be removed");
}

#[test]
fn a_rename_never_undoes_a_save_that_lands_beside_it() {
    let p = project();
    let made = create(&p.root, "Race", PlanPreset::Brainstorm).expect("create");
    let root = p.root.clone();
    let id = made.id.clone();
    let saver = std::thread::spawn(move || {
        let mut plan = get(&root, &id).expect("get").plan;
        for n in 0..40 {
            plan.nodes.push(note(&format!("n-{n:03}"), n, 0));
            plan = save(&root, &id, &plan).expect("save").plan;
        }
    });
    for n in 0..40 {
        rename(&p.root, &made.id, &format!("Race {n}")).expect("rename");
    }
    saver.join().expect("saver");
    let last = get(&p.root, &made.id).expect("get").plan;
    assert_eq!(last.nodes.len(), 40, "every save is kept");
    assert_eq!(last.name, "Race 39");
}

#[test]
fn a_file_with_untrimmed_fields_opens_as_it_would_save() {
    let p = project();
    let made = create(&p.root, "Loose", PlanPreset::Brainstorm).expect("create");
    let file = path(&p.root, &made.id).expect("path");
    let mut plan = made.plan.clone();
    plan.nodes = vec![
        PlanNode {
            title: " porch ".into(),
            ..note("n-2", 0, 0)
        },
        PlanNode {
            kind: NodeKind::Link,
            url: Some(" https://a.example ".into()),
            ..note("n-1", 0, 0)
        },
    ];
    plan.edges = vec![PlanEdge {
        label: Some(String::new()),
        ..edge("e-1", "n-1", "n-2")
    }];
    fs::write(&file, serde_json::to_string_pretty(&plan).expect("json")).expect("write");
    let opened = get(&p.root, &made.id).expect("get").plan;
    assert_eq!(opened.nodes[0].id, "n-1", "sorted");
    assert_eq!(opened.nodes[0].url.as_deref(), Some("https://a.example"));
    assert_eq!(opened.nodes[1].title, "porch");
    assert_eq!(opened.edges[0].label, None);
    // Sent back unchanged, it is the canvas on disk: nothing new to write.
    let before = read(&file);
    assert_eq!(save(&p.root, &made.id, &opened).expect("save").plan, opened);
    assert_eq!(read(&file), before);
}

#[test]
fn a_save_that_would_not_open_again_is_refused() {
    let p = project();
    let made = create(&p.root, "Heavy", PlanPreset::Brainstorm).expect("create");
    let mut plan = made.plan.clone();
    plan.nodes = (0..500)
        .map(|n| PlanNode {
            text: "x".repeat(TEXT_MAX_CHARS),
            ..note(&format!("n-{n:03}"), 0, 0)
        })
        .collect();
    let refused = save(&p.root, &made.id, &plan).expect_err("over the cap");
    assert!(refused.to_string().contains("too big to save"), "{refused}");
    assert!(
        get(&p.root, &made.id)
            .expect("still opens")
            .plan
            .nodes
            .is_empty()
    );
}

#[test]
fn the_console_and_superscript_device_names_are_refused_too() {
    for bad in [
        "CONIN$",
        "conout$.txt",
        "COM\u{b9}",
        "lpt\u{b3}.png",
        "concepts/COM\u{b2}/a.png",
    ] {
        assert!(!is_project_relative(bad), "{bad:?} must be refused");
    }
    assert!(is_project_relative("concepts/com10.png"));
}

#[test]
fn an_asset_node_keeps_only_the_asset_id() {
    let p = project();
    let made = create(&p.root, "Boss", PlanPreset::Brainstorm).expect("create");
    let asset = PlanNode {
        kind: NodeKind::Asset,
        text: String::new(),
        asset: Some("frost-warden".into()),
        ..note("n-1", 0, 0)
    };
    let direction = PlanNode {
        kind: NodeKind::Direction,
        text: String::new(),
        direction: Some("kotatsu-corner".into()),
        ..note("n-2", 0, 0)
    };
    let mut plan = made.plan.clone();
    plan.nodes = vec![asset.clone(), direction.clone()];
    save(&p.root, &made.id, &plan).expect("save");
    let text = read(&p.root.join(&made.file));
    assert!(text.contains("\"kind\": \"asset\""));
    assert!(text.contains("\"asset\": \"frost-warden\""));
    assert!(text.contains("\"direction\": \"kotatsu-corner\""));
    let back = get(&p.root, &made.id).expect("get").plan;
    assert_eq!(back.nodes, vec![asset.clone(), direction.clone()]);
    let listed = list(&p.root).expect("list");
    assert_eq!((listed[0].assets, listed[0].nodes), (1, 2));

    for (node, why) in [
        (
            PlanNode {
                asset: None,
                ..asset.clone()
            },
            "an asset node without an asset",
        ),
        (
            PlanNode {
                asset: Some("../x".into()),
                ..asset.clone()
            },
            "an asset id that names a path",
        ),
        (
            PlanNode {
                asset: Some("a".repeat(LINK_ID_MAX_CHARS + 1)),
                ..asset.clone()
            },
            "a long asset id",
        ),
        (
            PlanNode {
                title: "Frost Warden".into(),
                ..asset.clone()
            },
            "a copied name on an asset node",
        ),
        (
            PlanNode {
                url: Some("https://a.example".into()),
                ..asset.clone()
            },
            "an address on an asset node",
        ),
        (
            PlanNode {
                direction: Some("d".into()),
                ..asset.clone()
            },
            "a direction on an asset node",
        ),
        (
            PlanNode {
                asset: Some("a".into()),
                ..note("n-1", 0, 0)
            },
            "an asset on a note",
        ),
        (
            PlanNode {
                direction: None,
                ..direction.clone()
            },
            "a direction node without one",
        ),
        (
            PlanNode {
                text: "notes".into(),
                ..direction.clone()
            },
            "text on a direction node",
        ),
    ] {
        plan.nodes = vec![node];
        assert!(validate(&plan).is_err(), "{why} must be refused");
    }
}

#[test]
fn both_sides_trim_the_same_characters_so_a_padded_title_settles() {
    let p = project();
    let made = create(&p.root, "\u{feff}Trim\u{85}", PlanPreset::Brainstorm).expect("create");
    assert_eq!(made.plan.name, "Trim");
    let file = path(&p.root, &made.id).expect("path");
    let mut plan = made.plan.clone();
    plan.nodes = vec![PlanNode {
        title: "\u{feff} porch \u{85}".into(),
        ..note("n-1", 0, 0)
    }];
    fs::write(&file, serde_json::to_string_pretty(&plan).expect("json")).expect("write");
    let opened = get(&p.root, &made.id).expect("get").plan;
    assert_eq!(opened.nodes[0].title, "porch");
    let before = read(&file);
    save(&p.root, &made.id, &opened).expect("save");
    assert_eq!(read(&file), before, "nothing new to write");
}

#[test]
fn a_long_title_padded_with_spaces_is_checked_after_trimming() {
    let p = project();
    let made = create(&p.root, "Padded", PlanPreset::Brainstorm).expect("create");
    let file = path(&p.root, &made.id).expect("path");
    let full = "t".repeat(TITLE_MAX_CHARS);
    let mut plan = made.plan.clone();
    plan.nodes = vec![PlanNode {
        title: format!("   {full}   "),
        ..note("n-1", 0, 0)
    }];
    fs::write(&file, serde_json::to_string_pretty(&plan).expect("json")).expect("write");
    let listed = list(&p.root).expect("list");
    assert_eq!(listed[0].problem, None, "it opens, not listed as broken");
    assert_eq!(
        get(&p.root, &made.id).expect("get").plan.nodes[0].title,
        full
    );
    let saved = save(&p.root, &made.id, &plan).expect("a padded save is trimmed first");
    assert_eq!(saved.plan.nodes[0].title, full);
    plan.nodes[0].title = format!("{full}t");
    assert!(save(&p.root, &made.id, &plan).is_err(), "still too long");
}
