//! Concept jobs against a fake `codex` script: arrival, object refs, copy out,
//! failures, timeouts, cancelling, rate limits and Retry.

use super::sink::session_id;
use super::*;
use crate::concepts::objects::{self, NewObjectRefs};
use crate::concepts::{Aspect, load_direction, style};
use crate::pipeline::tools::ResolvedTool;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;

/// A fake `codex` that reads the prompt from stdin, checks the command
/// line, and acts out `mode`.
fn fake_codex(dir: &Path, placeholder: &Path, home: &Path, mode: &str) -> PathBuf {
    let script = format!(
        r#"#!/bin/sh
prompt=$(cat)
out=$(printf '%s\n' "$prompt" | grep -E '^/.*\.png$' | tail -n 1)
case "$prompt" in "Generate an image."*) ;; *) echo "bad prompt start" >&2; exit 9;; esac
[ "$1" = "exec" ] || {{ echo "not exec" >&2; exit 9; }}
for last; do :; done
[ "$last" = "-" ] || {{ echo "prompt not from stdin" >&2; exit 9; }}
echo "OpenAI Codex (fake)"
echo "session id: 01fake-$$"
echo "hook: SessionStart" >&2
mkdir -p "$(dirname "$out")"
n=$(cat "$out.count" 2>/dev/null || echo 0); n=$((n + 1)); echo "$n" > "$out.count"
case "{mode}" in
  ok) mkdir -p "$(dirname "$out")"; cp "{png}" "$out"; echo "saved $out";;
  record) mkdir -p "$(dirname "$out")"; cp "{png}" "$out"; printf '%s\n' "$@" > "$out.args"; printf '%s\n' "$prompt" > "$out.prompt";;
  leave) mkdir -p "{home}/generated_images/01fake-$$"; cp "{png}" "{home}/generated_images/01fake-$$/exec-1.png"; echo "image ready";;
  nothing) echo "I could not make that.";;
  fail) echo "error: not signed in" >&2; exit 2;;
  slow) sleep 30;;
  limited) echo "ERROR: exceeded retry limit, last status: 429 Too Many Requests" >&2; exit 1;;
  limited-once) if [ "$n" = 1 ]; then echo "ERROR: exceeded retry limit, last status: 429 Too Many Requests" >&2; exit 1; fi; cp "{png}" "$out";;
  usage) if [ ! -f "{home}/lifted" ]; then echo "ERROR: You’ve hit your usage limit. Upgrade to Pro (https://chatgpt.com/explore/pro) or try again later." >&2; exit 1; fi; cp "{png}" "$out";;
  flaky) if [ "$n" = 1 ]; then echo "error: not signed in" >&2; exit 2; fi; cp "{png}" "$out";;
  flaky-record) if [ "$n" = 1 ]; then echo "error: not signed in" >&2; exit 2; fi; cp "{png}" "$out"; printf '%s\n' "$@" > "$out.args"; printf '%s\n' "$prompt" > "$out.prompt";;
esac
"#,
        mode = mode,
        png = placeholder.display(),
        home = home.display()
    );
    let path = dir.join(format!("codex-{mode}"));
    fs::write(&path, script).expect("script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

fn toolbox(codex: &Path) -> Toolbox {
    Toolbox {
        tools: BTreeMap::from([(
            ToolId::Codex,
            ResolvedTool {
                program: codex.to_path_buf(),
                prefix: Vec::new(),
                version: "0.0.0-fake".into(),
            },
        )]),
        path_env: std::env::var_os("PATH").unwrap_or_default(),
        os: Some(crate::toolchain::HostOs::current()),
    }
}

fn request(direction: &str, count: u32) -> NewConcepts {
    NewConcepts {
        project: "moon".into(),
        direction: direction.into(),
        prompt: "A cosy porch at dusk.".into(),
        refs: Vec::new(),
        aspect: Aspect::Wide,
        count,
        use_style: true,
    }
}

struct Rig {
    _dir: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    cache: PathBuf,
    png: PathBuf,
    home: PathBuf,
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    let project = root.join("moon");
    let cache = root.join("cache/concepts");
    let home = root.join("codex-home");
    fs::create_dir_all(&project).expect("project");
    let png = root.join("placeholder.png");
    fs::write(
        &png,
        crate::inbox::images::write_png(8, 4, &[120; 8 * 4 * 3]),
    )
    .expect("png");
    Rig {
        _dir: dir,
        root,
        project,
        cache,
        png,
        home,
    }
}

fn events() -> (
    Arc<Mutex<Vec<ConceptEvent>>>,
    impl Fn(ConceptEvent) + Send + Sync,
) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    (log, move |e| sink.lock().expect("events").push(e))
}

#[test]
fn images_arrive_by_file_and_land_in_the_direction() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "ok");
    let (log, on_event) = events();
    let concepts = Concepts::new(Some(r.home.clone()), 2, on_event);
    let (direction, jobs) = concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&codex),
            request: request("Porch at dusk", 3),
            timeout: Duration::from_secs(20),
        })
        .expect("start");
    assert_eq!(jobs.len(), 3);
    assert_eq!(direction.id, "porch-at-dusk");
    concepts.wait();
    let done = concepts.jobs(Some("moon"));
    assert!(
        done.iter().all(|j| j.state == ConceptJobState::Done),
        "{done:#?}"
    );
    assert_eq!((done[0].width, done[0].height), (Some(8), Some(4)));
    assert!(done[0].log_tail.iter().any(|l| l.starts_with("saved ")));
    let stored = load_direction(&r.project, "porch-at-dusk")
        .expect("load")
        .expect("direction");
    assert_eq!(stored.images.len(), 3);
    assert!(
        r.project
            .join("concepts/porch-at-dusk/porch-at-dusk_03.png")
            .is_file()
    );
    let events = log.lock().expect("events");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ConceptEvent::Direction { .. }))
    );
    assert!(
        events.iter().any(
            |e| matches!(e, ConceptEvent::Job { job } if job.state == ConceptJobState::Running)
        )
    );
}

#[test]
fn object_refs_get_one_titled_job_per_object_with_the_source_attached() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "record");
    let source = r
        .project
        .join("concepts/workbench-hero/workbench-hero_01.png");
    fs::create_dir_all(source.parent().expect("parent")).expect("mkdir");
    fs::copy(&r.png, &source).expect("source");
    let concepts = Concepts::new(Some(r.home.clone()), 2, |_| {});
    let tools = toolbox(&codex);
    let lane = Lane {
        project_dir: &r.project,
        cache_dir: &r.cache,
        tools: &tools,
        timeout: Duration::from_secs(20),
    };
    let request = objects::clean(
        &r.project,
        &NewObjectRefs {
            project: "moon".into(),
            source: "concepts/workbench-hero/workbench-hero_01.png".into(),
            source_name: "Workbench hero".into(),
            objects: vec!["desk lamp".into(), "plaster plinth with clay figure".into()],
            notes: std::collections::HashMap::from([(
                "Desk lamp".to_string(),
                "brass, warm bulb".to_string(),
            )]),
            aspect: Aspect::Square,
            kind: crate::assets::AssetKind::Prop,
            use_style: true,
        },
    )
    .expect("clean");
    let (direction, jobs) = concepts.start_objects(&lane, &request).expect("start");
    assert_eq!(direction.name, "Workbench hero objects");
    assert_eq!(
        jobs.iter().map(|j| j.title.as_deref()).collect::<Vec<_>>(),
        vec![Some("desk lamp"), Some("plaster plinth with clay figure")]
    );
    concepts.wait();
    let done = concepts.jobs(Some("moon"));
    assert!(
        done.iter().all(|j| j.state == ConceptJobState::Done),
        "{done:#?}"
    );
    let stored = load_direction(&r.project, "workbench-hero-objects")
        .expect("load")
        .expect("direction");
    assert_eq!(
        stored
            .images
            .iter()
            .map(|i| i.title.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("desk lamp"), Some("plaster plinth with clay figure")]
    );
    assert_eq!(
        stored.source.as_deref(),
        Some("concepts/workbench-hero/workbench-hero_01.png")
    );
    let first = r
        .project
        .join("concepts/workbench-hero-objects/workbench-hero-objects_01.png");
    let prompt = fs::read_to_string(first.with_extension("png.prompt")).expect("prompt");
    assert!(prompt.contains("the \"desk lamp\" from the attached concept image"));
    assert!(prompt.contains("plain light grey background"));
    assert!(prompt.contains("Extra instructions for this object: brass, warm bulb"));
    let args = fs::read_to_string(first.with_extension("png.args")).expect("args");
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(
        args[..4],
        ["exec", "--skip-git-repo-check", "-s", "workspace-write"]
    );
    assert_eq!(args[4], "-i");
    assert_eq!(args[5], source.display().to_string());
    assert_eq!(args[6..], ["--", "-"]);
    let second = fs::read_to_string(
        r.project
            .join("concepts/workbench-hero-objects/workbench-hero-objects_02.png.prompt"),
    )
    .expect("second prompt");
    assert!(second.contains("Show only the plaster plinth with clay figure, isolated"));
    assert!(!second.contains("Extra instructions"));
}

#[test]
fn object_refs_started_at_the_same_moment_never_share_a_file() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "record");
    let source = r
        .project
        .join("concepts/workbench-hero/workbench-hero_01.png");
    fs::create_dir_all(source.parent().expect("parent")).expect("mkdir");
    fs::copy(&r.png, &source).expect("source");
    let concepts = Concepts::new(Some(r.home.clone()), 4, |_| {});
    let tools = toolbox(&codex);
    let lane = Lane {
        project_dir: &r.project,
        cache_dir: &r.cache,
        tools: &tools,
        timeout: Duration::from_secs(20),
    };
    let ask = |object: &str| NewObjectRefs {
        project: "moon".into(),
        source: "concepts/workbench-hero/workbench-hero_01.png".into(),
        source_name: "Workbench hero".into(),
        objects: vec![object.into()],
        notes: Default::default(),
        aspect: Aspect::Square,
        kind: crate::assets::AssetKind::Prop,
        use_style: true,
    };
    let gate = std::sync::Barrier::new(2);
    let started: Vec<Vec<ConceptJob>> = std::thread::scope(|s| {
        let calls: Vec<_> = ["armillary sphere", "brass astrolabe"]
            .into_iter()
            .map(|object| {
                let (gate, lane, concepts, request) = (&gate, &lane, &concepts, ask(object));
                s.spawn(move || {
                    gate.wait();
                    concepts.start_objects(lane, &request).expect("start").1
                })
            })
            .collect();
        calls
            .into_iter()
            .map(|c| c.join().expect("thread"))
            .collect()
    });
    let files: Vec<&str> = started.iter().flatten().map(|j| j.file.as_str()).collect();
    assert_eq!(files.len(), 2);
    assert_ne!(files[0], files[1], "both calls got {}", files[0]);
    concepts.wait();
    let stored = load_direction(&r.project, "workbench-hero-objects")
        .expect("load")
        .expect("direction");
    let mut titles: Vec<_> = stored
        .images
        .iter()
        .filter_map(|i| i.title.clone())
        .collect();
    titles.sort();
    assert_eq!(titles, ["armillary sphere", "brass astrolabe"]);
    for job in started.iter().flatten() {
        let prompt =
            fs::read_to_string(r.project.join(format!("{}.prompt", job.file))).expect("prompt");
        let object = job.title.as_deref().expect("title");
        assert!(
            prompt.contains(&format!("the \"{object}\" from")),
            "{prompt}"
        );
    }
}

#[test]
fn an_image_left_in_codex_home_is_copied_out() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "leave");
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&codex),
            request: request("Lantern", 1),
            timeout: Duration::from_secs(20),
        })
        .expect("start");
    concepts.wait();
    let job = &concepts.jobs(None)[0];
    assert_eq!(job.state, ConceptJobState::Done, "{job:#?}");
    assert!(job.message.as_deref().is_some_and(|m| m.contains("copied")));
    assert!(r.project.join("concepts/lantern/lantern_01.png").is_file());
}

#[test]
fn failures_and_timeouts_say_what_happened() {
    let r = rig();
    let concepts = Concepts::new(Some(r.home.clone()), 3, |_| {});
    for (mode, direction, timeout) in [
        ("nothing", "Empty", 20),
        ("fail", "Broken", 20),
        ("slow", "Slow", 1),
    ] {
        let codex = fake_codex(&r.root, &r.png, &r.home, mode);
        concepts
            .start(Start {
                project_dir: &r.project,
                cache_dir: &r.cache,
                tools: &toolbox(&codex),
                request: request(direction, 1),
                timeout: Duration::from_secs(timeout),
            })
            .expect("start");
    }
    concepts.wait();
    let jobs = concepts.jobs(Some("moon"));
    let message = |d: &str| {
        let job = jobs.iter().find(|j| j.direction == d).expect("job");
        assert_eq!(job.state, ConceptJobState::Failed, "{job:#?}");
        job.message.clone().unwrap_or_default()
    };
    assert!(message("empty").contains("without saving the image"));
    let broken = message("broken");
    assert!(broken.contains("exited with code 2"), "{broken}");
    assert!(broken.contains("not signed in"), "{broken}");
    assert!(message("slow").contains("ran longer than"));
}

#[test]
fn queued_jobs_can_be_cancelled() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "slow");
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    let (_, jobs) = concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&codex),
            request: request("Slow", 2),
            timeout: Duration::from_secs(60),
        })
        .expect("start");
    concepts.cancel(&jobs[1].id).expect("cancel queued");
    assert_eq!(concepts.cancel_all("moon"), 1);
    concepts.wait();
    assert!(
        concepts
            .jobs(None)
            .iter()
            .all(|j| j.state == ConceptJobState::Cancelled)
    );
    concepts.clear_finished("moon");
    assert!(concepts.jobs(None).is_empty());
    // Without Codex nothing starts.
    let err = concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &Toolbox::default(),
            request: request("Slow", 1),
            timeout: Duration::from_secs(60),
        })
        .expect_err("no codex");
    assert!(err.to_string().contains("Codex CLI"));
}

fn quick(tries: u32, first_ms: u64) -> Backoff {
    Backoff {
        first: Duration::from_millis(first_ms),
        longest: Duration::from_secs(1),
        tries,
    }
}

fn start_one(concepts: &Concepts, r: &Rig, codex: &Path, direction: &str) -> ConceptJob {
    let (_, jobs) = concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(codex),
            request: request(direction, 1),
            timeout: Duration::from_secs(20),
        })
        .expect("start");
    jobs.into_iter().next().expect("job")
}

#[test]
fn a_rate_limited_job_waits_and_runs_again_on_its_own() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "limited-once");
    let (log, on_event) = events();
    let concepts = Concepts::new(Some(r.home.clone()), 1, on_event);
    concepts.set_backoff(quick(3, 50));
    let first = start_one(&concepts, &r, &codex, "Lantern");
    concepts.wait();
    let job = &concepts.jobs(None)[0];
    assert_eq!(job.id, first.id);
    assert_eq!(job.state, ConceptJobState::Done, "{job:#?}");
    assert_eq!(job.retries, 1);
    assert!(
        job.log_tail.iter().all(|l| !l.starts_with("hook:")),
        "{job:#?}"
    );
    let events = log.lock().expect("events");
    let waiting = events
        .iter()
        .find_map(|e| match e {
            ConceptEvent::Job { job } if job.state == ConceptJobState::RateLimited => Some(job),
            _ => None,
        })
        .expect("a waiting job");
    assert!(waiting.retry_at_ms.is_some());
    assert_eq!(
        waiting.message.as_deref(),
        Some("Rate limited by Codex. Trying again on its own (try 2 of 4).")
    );
    let stored = load_direction(&r.project, "lantern")
        .expect("load")
        .expect("direction");
    assert_eq!(stored.images.len(), 1);
}

#[test]
fn a_job_rate_limited_every_time_stops_after_its_tries_with_a_plain_message() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "limited");
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    concepts.set_backoff(quick(2, 20));
    start_one(&concepts, &r, &codex, "Lantern");
    concepts.wait();
    let job = &concepts.jobs(None)[0];
    assert_eq!(job.state, ConceptJobState::Failed, "{job:#?}");
    assert_eq!(job.retries, 2);
    assert_eq!(job.retry_at_ms, None);
    assert_eq!(
        job.message.as_deref(),
        Some("Rate limited by Codex, 3 times in a row. Try again in a few minutes with Retry.")
    );
    let count =
        fs::read_to_string(r.project.join("concepts/lantern/lantern_01.png.count")).expect("count");
    assert_eq!(count.trim(), "3");
}

#[test]
fn retry_runs_a_failed_job_again_with_the_same_file_and_no_new_direction() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "flaky");
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    let first = start_one(&concepts, &r, &codex, "Lantern");
    concepts.wait();
    let failed = &concepts.jobs(None)[0];
    assert_eq!(failed.state, ConceptJobState::Failed, "{failed:#?}");
    assert!(
        failed
            .message
            .as_deref()
            .is_some_and(|m| !m.contains("hook:"))
    );
    let again = concepts.retry(&first.id).expect("retry");
    assert_eq!(again.id, first.id);
    assert_eq!(again.file, first.file);
    assert_ne!(again.state, ConceptJobState::Failed);
    concepts.wait();
    let jobs = concepts.jobs(None);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, ConceptJobState::Done, "{jobs:#?}");
    assert_eq!(jobs[0].message, None);
    let directions = fs::read_dir(r.project.join("concepts"))
        .expect("concepts")
        .count();
    assert_eq!(directions, 1);
    // A done job does not run again, and an unknown one is refused.
    let done = concepts.retry(&first.id).expect_err("done");
    assert!(done.to_string().contains("is done"), "{done}");
    assert!(concepts.retry("nope").is_err());
}

#[test]
fn a_job_waiting_out_a_rate_limit_can_be_cancelled() {
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "limited");
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    concepts.set_backoff(quick(3, 60_000));
    let first = start_one(&concepts, &r, &codex, "Lantern");
    let second = start_one(&concepts, &r, &codex, "Porch");
    let waiting = || {
        concepts
            .jobs(None)
            .iter()
            .filter(|j| j.state == ConceptJobState::RateLimited)
            .count()
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    while waiting() < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(waiting(), 2, "{:#?}", concepts.jobs(None));
    concepts.cancel(&first.id).expect("cancel");
    assert_eq!(concepts.cancel_all("moon"), 1);
    concepts.wait();
    for job in concepts.jobs(None) {
        assert_eq!(job.state, ConceptJobState::Cancelled, "{job:#?}");
        assert_eq!(job.retry_at_ms, None);
    }
    // Retry runs a cancelled one again at once.
    let again = concepts.retry(&second.id).expect("retry");
    assert_eq!(again.retries, 0);
    concepts.cancel(&second.id).expect("cancel again");
    concepts.wait();
}

#[test]
fn a_usage_limit_holds_the_whole_queue_until_it_lifts() {
    let r = rig();
    fs::create_dir_all(&r.home).expect("home");
    let codex = fake_codex(&r.root, &r.png, &r.home, "usage");
    let (log, on_event) = events();
    let concepts = Concepts::new(Some(r.home.clone()), 1, on_event);
    // No time in the message: the rate limit backoff decides the wait.
    concepts.set_backoff(quick(3, 1000));
    let (_, jobs) = concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&codex),
            request: request("Lantern", 3),
            timeout: Duration::from_secs(20),
        })
        .expect("start");
    let usage_waiting = || {
        concepts
            .jobs(None)
            .iter()
            .filter(|j| j.state == ConceptJobState::RateLimited && j.usage_limit)
            .count()
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    while usage_waiting() < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(usage_waiting() >= 2, "{:#?}", concepts.jobs(None));
    // A job started while the limit holds waits for it too.
    let late = start_one(&concepts, &r, &codex, "Porch");
    assert_eq!(late.state, ConceptJobState::RateLimited, "{late:#?}");
    assert!(late.usage_limit);
    fs::write(r.home.join("lifted"), "").expect("lift");
    concepts.wait();
    for job in concepts.jobs(None) {
        assert_eq!(job.state, ConceptJobState::Done, "{job:#?}");
        assert!(!job.usage_limit);
    }
    // The last queued job never ran into the limit: it waited, then ran once.
    let last = &jobs[2];
    let count = fs::read_to_string(r.project.join(format!("{}.count", last.file))).expect("count");
    assert_eq!(count.trim(), "1");
    let waiting = log
        .lock()
        .expect("events")
        .iter()
        .find_map(|e| match e {
            ConceptEvent::Job { job } if job.usage_limit => job.message.clone(),
            _ => None,
        })
        .expect("a usage wait");
    assert!(
        waiting.starts_with("Waiting for the Codex usage limit until about "),
        "{waiting}"
    );
}

#[test]
fn a_usage_read_moves_the_held_jobs_to_its_reset_and_lift_runs_them_now() {
    let r = rig();
    fs::create_dir_all(&r.home).expect("home");
    let codex = fake_codex(&r.root, &r.png, &r.home, "usage");
    let (_log, on_event) = events();
    let concepts = Concepts::new(Some(r.home.clone()), 1, on_event);
    // A long backoff: nothing runs again on its own during the test.
    concepts.set_backoff(quick(3, 60_000));
    concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&codex),
            request: request("Lantern", 2),
            timeout: Duration::from_secs(20),
        })
        .expect("start");
    let deadline = Instant::now() + Duration::from_secs(20);
    while concepts.usage_held().len() < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(concepts.usage_held().len(), 2, "{:#?}", concepts.jobs(None));
    // The read's reset time beats the backoff guess.
    let resets = now_ms() + 3_600_000.0;
    let moved = concepts.retime_usage_limit(resets);
    assert_eq!(moved.len(), 2);
    for job in concepts.usage_held() {
        assert_eq!(job.retry_at_ms, Some(resets + 60_000.0), "{job:#?}");
        assert!(
            job.message
                .as_deref()
                .is_some_and(|m| m.contains("until about"))
        );
    }
    // The same time again moves nothing, and a past one is ignored.
    assert!(concepts.retime_usage_limit(resets).is_empty());
    assert!(concepts.retime_usage_limit(now_ms() - 120_000.0).is_empty());
    // A job started now waits for the new time too.
    let late = start_one(&concepts, &r, &codex, "Porch");
    assert_eq!(late.retry_at_ms, Some(resets + 60_000.0));
    // Lifted: every held job runs at once instead of in an hour.
    fs::write(r.home.join("lifted"), "").expect("lift");
    let requeued = concepts.lift_usage_limit();
    assert_eq!(requeued.len(), 3);
    assert!(requeued.iter().all(|j| !j.usage_limit));
    concepts.wait();
    for job in concepts.jobs(None) {
        assert_eq!(job.state, ConceptJobState::Done, "{job:#?}");
    }
    assert!(concepts.usage_held().is_empty());
}

#[test]
fn tells_rate_limits_and_hook_lines_apart_from_other_lines() {
    use super::retry::{is_noise, says_rate_limited};
    assert!(says_rate_limited(
        "ERROR: exceeded retry limit, last status: 429 Too Many Requests"
    ));
    assert!(says_rate_limited(
        "stream error: Rate limit reached for requests"
    ));
    assert!(!says_rate_limited("error: not signed in"));
    assert!(!says_rate_limited("saved /tmp/a_429.png"));
    assert!(is_noise("hook: SessionStart Completed"));
    assert!(!is_noise("ERROR: hook failed"));
}

#[test]
fn reads_the_session_id_from_the_header() {
    assert_eq!(
        session_id("session id: 01a16a3c-fd0b-7983-bfe5-5ec6b314d77e").as_deref(),
        Some("01a16a3c-fd0b-7983-bfe5-5ec6b314d77e")
    );
    assert_eq!(session_id("Session ID: ../etc"), None);
    assert_eq!(session_id("workdir: /tmp"), None);
}

#[test]
fn jobs_carry_the_style_board_and_a_retry_carries_the_same_one() {
    let r = rig();
    fs::create_dir_all(r.project.join("refs")).expect("refs");
    fs::copy(&r.png, r.project.join("refs/frost_glass.png")).expect("ref");
    let board = style::update(
        &r.project,
        &style::StyleBoardChange {
            line: Some("frosted glass, cool night light".into()),
            palette: Some(vec!["#9fc3ff".into()]),
            images: Some(vec!["refs/frost_glass.png".into()]),
        },
    )
    .expect("board");
    assert_eq!(board.revision, 1);
    let codex = fake_codex(&r.root, &r.png, &r.home, "flaky-record");
    let concepts = Concepts::new(Some(r.home.clone()), 1, |_| {});
    let first = start_one(&concepts, &r, &codex, "Lantern");
    assert_eq!(first.style_revision, Some(1));
    let stored = load_direction(&r.project, "lantern")
        .expect("load")
        .expect("direction");
    assert_eq!((stored.style_revision, stored.style_off), (Some(1), false));
    concepts.wait();
    // The board changes before the retry; the retry still carries revision 1.
    style::update(
        &r.project,
        &style::StyleBoardChange {
            line: Some("warm candle light".into()),
            ..Default::default()
        },
    )
    .expect("second board");
    let again = concepts.retry(&first.id).expect("retry");
    assert_eq!(again.style_revision, Some(1));
    concepts.wait();
    let out = r.project.join(&first.file);
    let prompt = fs::read_to_string(out.with_extension("png.prompt")).expect("prompt");
    assert!(
        prompt.contains("Style: frosted glass, cool night light."),
        "{prompt}"
    );
    assert!(!prompt.contains("candle"));
    let args = fs::read_to_string(out.with_extension("png.args")).expect("args");
    let args: Vec<&str> = args.lines().collect();
    let copy = r
        .project
        .join("style/frost_glass.png")
        .display()
        .to_string();
    assert_eq!(args[args.len() - 3], copy);

    // Turned off for one direction, the next start carries nothing.
    let plain = fake_codex(&r.root, &r.png, &r.home, "record");
    let mut off = request("Lantern off", 1);
    off.use_style = false;
    let (direction, jobs) = concepts
        .start(Start {
            project_dir: &r.project,
            cache_dir: &r.cache,
            tools: &toolbox(&plain),
            request: off,
            timeout: Duration::from_secs(20),
        })
        .expect("start");
    assert!(direction.style_off);
    assert_eq!(direction.style_revision, None);
    assert_eq!(jobs[0].style_revision, None);
    concepts.wait();
    let out = r.project.join(&jobs[0].file);
    let prompt = fs::read_to_string(out.with_extension("png.prompt")).expect("prompt");
    assert!(!prompt.contains("Project style"));
    let args = fs::read_to_string(out.with_extension("png.args")).expect("args");
    assert!(!args.contains("frost_glass"));
}

#[test]
fn other_views_draw_only_the_empty_slots_with_the_front_attached() {
    use crate::refs::{AssetRef, RefView};
    let r = rig();
    let codex = fake_codex(&r.root, &r.png, &r.home, "record");
    for f in ["refs/frost_lantern.png", "refs/lantern_left.png"] {
        fs::create_dir_all(r.project.join("refs")).expect("refs");
        fs::copy(&r.png, r.project.join(f)).expect("ref");
    }
    let asset: crate::Asset = serde_json::from_str(
        r#"{"id":"frost-lantern","name":"Frost lantern","kind":"prop","stage":"refs",
            "status":"queued","versions":[],"thumbnail":null,"createdAt":"t","updatedAt":"t"}"#,
    )
    .expect("asset");
    let refs = vec![
        AssetRef {
            file: "refs/frost_lantern.png".into(),
            view: RefView::Front,
        },
        AssetRef {
            file: "refs/lantern_left.png".into(),
            view: RefView::Left,
        },
    ];
    let concepts = Concepts::new(Some(r.home.clone()), 2, |_| {});
    let tools = toolbox(&codex);
    let lane = Lane {
        project_dir: &r.project,
        cache_dir: &r.cache,
        tools: &tools,
        timeout: Duration::from_secs(20),
    };
    let mut request = crate::concepts::NewOtherViews {
        project: "moon".into(),
        asset: "frost-lantern".into(),
        views: Vec::new(),
        count: 2,
        use_style: true,
    };
    let (direction, jobs) = concepts
        .start_views(&lane, &asset, &refs, &request)
        .expect("start");
    assert_eq!(direction.name, "Frost lantern views");
    assert_eq!(
        jobs.iter().map(|j| j.title.as_deref()).collect::<Vec<_>>(),
        vec![Some("back"), Some("back"), Some("right"), Some("right")],
        "left is filled, so it is skipped"
    );
    concepts.wait();
    let first = r
        .project
        .join("concepts/frost-lantern-views/frost-lantern-views_01.png");
    let prompt = fs::read_to_string(first.with_extension("png.prompt")).expect("prompt");
    assert!(prompt.contains("Draw the Frost lantern from behind it"));
    let args = fs::read_to_string(first.with_extension("png.args")).expect("args");
    assert!(
        args.contains("refs/frost_lantern.png"),
        "the front goes along"
    );
    let stored = load_direction(&r.project, "frost-lantern-views")
        .expect("load")
        .expect("direction");
    assert_eq!(
        stored.refs_job.map(|j| j.asset).as_deref(),
        Some("frost-lantern")
    );

    request.views = vec![RefView::Left];
    request.count = 1;
    let (_, again) = concepts
        .start_views(&lane, &asset, &refs, &request)
        .expect("redraw");
    assert_eq!(
        again[0].title.as_deref(),
        Some("left"),
        "a filled one on request"
    );
    concepts.wait();
    request.views = vec![RefView::Front];
    assert!(
        concepts
            .start_views(&lane, &asset, &refs, &request)
            .is_err()
    );
}
