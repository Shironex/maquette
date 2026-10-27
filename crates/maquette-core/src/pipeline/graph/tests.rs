use super::*;
use crate::pipeline::template;
use crate::project::ProjectTemplate;

fn step(id: &str, deps: &[&str]) -> StepDef {
    StepDef {
        id: id.into(),
        name: id.to_uppercase(),
        kind: StepKind::Shell,
        command: Some("sh".into()),
        script: None,
        args: vec!["-c".into(), "true".into()],
        inputs: Vec::new(),
        outputs: vec![format!("out/{id}.txt")],
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
        compress: None,
    }
}

fn def(steps: Vec<StepDef>) -> PipelineDef {
    PipelineDef {
        version: 1,
        root: None,
        steps,
        presets: Vec::new(),
    }
}

fn errors(def: &PipelineDef) -> Vec<String> {
    validate(def)
        .into_iter()
        .filter(PipelineIssue::is_error)
        .map(|i| i.message)
        .collect()
}

#[test]
fn the_templates_are_valid() {
    for t in [
        ProjectTemplate::FloatingDiorama,
        ProjectTemplate::Room,
        ProjectTemplate::CharacterShowcase,
        ProjectTemplate::Imported,
    ] {
        let d = template::for_template(t);
        assert!(errors(&d).is_empty(), "{t:?}: {:?}", errors(&d));
    }
}

#[test]
fn missing_and_self_dependencies() {
    let d = def(vec![step("a", &["ghost"]), step("b", &["b"])]);
    let errs = errors(&d);
    assert!(
        errs.iter().any(|e| e.contains("depends on ghost")),
        "{errs:?}"
    );
    assert!(
        errs.iter().any(|e| e.contains("depends on itself")),
        "{errs:?}"
    );
}

#[test]
fn cycles_are_found_and_named() {
    let d = def(vec![
        step("a", &[]),
        step("b", &["a", "d"]),
        step("c", &["b"]),
        step("d", &["c"]),
    ]);
    assert_eq!(
        topo_order(&d).expect_err("cycle"),
        vec!["b".to_string(), "c".to_string(), "d".to_string()]
    );
    assert!(errors(&d).iter().any(|e| e.contains("loop: b, c, d")));
    assert!(plan(&d, None, false).is_err());
}

#[test]
fn duplicate_and_bad_ids() {
    let d = def(vec![step("a", &[]), step("a", &[]), step("x/y", &[])]);
    let errs = errors(&d);
    assert!(errs.iter().any(|e| e.contains("share the id a")));
    assert!(errs.iter().any(|e| e.contains("not a step id")));
}

#[test]
fn order_respects_dependencies_and_keeps_definition_order() {
    let d = def(vec![
        step("ship", &["compress"]),
        step("build", &[]),
        step("compress", &["export"]),
        step("export", &["build"]),
        step("posters", &["build"]),
    ]);
    let ids: Vec<&str> = topo_order(&d)
        .expect("order")
        .into_iter()
        .map(|i| d.steps[i].id.as_str())
        .collect();
    assert_eq!(ids, ["build", "export", "compress", "ship", "posters"]);
}

#[test]
fn run_from_here_forces_the_step_and_runs_its_dependents() {
    let d = def(vec![
        step("build", &[]),
        step("bake", &["build"]),
        step("export", &["bake"]),
        step("posters", &["build"]),
        step("compress", &["export"]),
    ]);
    let plan_of = |from, force| -> Vec<(String, Planned)> {
        plan(&d, from, force)
            .expect("plan")
            .into_iter()
            .map(|(i, p)| (d.steps[i].id.clone(), p))
            .collect()
    };
    assert_eq!(
        plan_of(Some("bake"), false),
        vec![
            ("build".into(), Planned::Skip),
            ("bake".into(), Planned::Force),
            ("export".into(), Planned::Run),
            ("posters".into(), Planned::Skip),
            ("compress".into(), Planned::Run),
        ]
    );
    assert!(
        plan_of(None, true)
            .iter()
            .all(|(_, p)| *p == Planned::Force)
    );
    assert!(plan_of(None, false).iter().all(|(_, p)| *p == Planned::Run));
    assert_eq!(
        plan(&d, Some("nope"), false).expect_err("missing").code(),
        "not-found"
    );
}

#[test]
fn blender_must_be_headless() {
    let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(blender_headless(&args(&[
        "-b", "a.blend", "--python", "x.py"
    ])));
    assert!(blender_headless(&args(&[
        "--factory-startup",
        "--background",
        "a.blend"
    ])));
    assert!(!blender_headless(&args(&["a.blend", "-b"])));
    assert!(!blender_headless(&args(&["--python", "x.py", "-b"])));
    assert!(!blender_headless(&args(&["--", "-b"])));
    assert!(!blender_headless(&args(&["--factory-startup"])));

    let mut s = step("build", &[]);
    s.kind = StepKind::Blender;
    s.args = args(&["models/a.blend", "--python", "tools/b.py"]);
    assert!(
        errors(&def(vec![s]))
            .iter()
            .any(|e| e.contains("always run headless"))
    );
}

#[test]
fn placeholders_paths_and_kinds() {
    assert_eq!(
        placeholders("{root}/a/{siteRepo}"),
        vec!["root", "siteRepo"]
    );
    assert_eq!(placeholders("{\"json\": 1}"), Vec::<&str>::new());
    assert_eq!(placeholders("{}"), Vec::<&str>::new());

    let mut s = step("a", &[]);
    s.args = vec!["{nope}".into(), "{script}".into()];
    s.inputs = vec!["../outside/*.blend".into(), "/abs/x".into()];
    s.command = None;
    let errs = errors(&def(vec![s]));
    assert!(
        errs.iter()
            .any(|e| e.contains("{nope} is not a placeholder"))
    );
    assert!(errs.iter().any(|e| e.contains("has no script")));
    assert!(errs.iter().any(|e| e.contains("../outside")));
    assert!(errs.iter().any(|e| e.contains("/abs/x")));
    assert!(errs.iter().any(|e| e.contains("without a command")));

    let mut c = step("compress", &[]);
    c.compress = Some(super::super::CompressSpec {
        from: "web/raw".into(),
        to: "web/dist".into(),
    });
    assert!(
        errors(&def(vec![c.clone()]))
            .iter()
            .any(|e| e.contains("only gltf-transform"))
    );
    c.kind = StepKind::GltfTransform;
    assert!(errors(&def(vec![c])).is_empty());
}

#[test]
fn a_build_only_run_leaves_the_ship_step_out() {
    let d = def(vec![
        step("build", &[]),
        step("ship", &["build"]),
        step("after", &["ship"]),
    ]);
    let full = plan_run(&d, None, false, false).expect("plan");
    assert!(full.iter().all(|(_, p)| *p == Planned::Run));
    let built = plan_run(&d, None, false, true).expect("plan");
    let ship = built
        .iter()
        .find(|(i, _)| d.steps[*i].id == "ship")
        .expect("ship");
    assert_eq!(ship.1, Planned::Skip);
    let after = built
        .iter()
        .find(|(i, _)| d.steps[*i].id == "after")
        .expect("after");
    assert_eq!(after.1, Planned::Run, "only Ship itself is left out");
}
