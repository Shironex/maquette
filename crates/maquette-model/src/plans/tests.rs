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
        by: None,
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
    let text = read(&file).replace("\"version\": 1", "\"version\": 9");
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
