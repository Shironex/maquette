//! Adding a reviewed bake as the model's next version: what refuses it,
//! the record it leaves on the new version and the staged model it clears.
//! The shell's route (the Inbox for the asset, then Accept) is played here
//! by dropping the staged model in the Inbox and accepting it.

use std::fs;
use std::path::{Path, PathBuf};

use super::tests::{Project, bakes, humanoid, point, reach, reaching, settled};
use super::*;
use crate::error::CoreResult;
use crate::pipeline::tools::Toolbox;
use crate::recipes::{self, baked};

/// A finished bake of `p`'s recipe, as Apply leaves it.
fn baked_once(p: &Project) -> (RecipeBakes, RecipeBakeRecord) {
    let (bakes, _) = bakes(false);
    bakes
        .start(
            p.root(),
            &p.config,
            Toolbox::default(),
            &p.request(),
            RecipeBakeStarter::Person,
        )
        .expect("start");
    let done = settled(&bakes, p);
    assert_eq!(done.state, RecipeBakeState::Done, "{:?}", done.error);
    (bakes, done)
}

/// Brings `file` in as the next version of Elsheria the way the shell
/// does after its own file check: into the Inbox, then accepted.
fn accept(p: &Project, file: &Path) -> u32 {
    let inbox = crate::inbox::inbox_dir(p.root(), &p.config);
    fs::create_dir_all(&inbox).expect("inbox");
    fs::copy(file, inbox.join("bake.glb")).expect("copy");
    crate::inbox::accept(
        p.root(),
        &p.config,
        &crate::inbox::AcceptRequest {
            item: "bake.glb".into(),
            project: p.id.clone(),
            asset: Some("elsheria".into()),
            name: None,
            prep: None,
        },
    )
    .expect("accept")
    .version
    .version
}

fn claim(bakes: &RecipeBakes, p: &Project, viewed: Option<f64>) -> CoreResult<AddClaim> {
    bakes.claim_add(p.root(), &p.config, &p.id, &p.recipe.id, viewed)
}

fn refused(p: &Project) -> String {
    match ready_to_add(p.root(), &p.config, &p.id, &p.recipe.id) {
        Ok(_) => panic!("not refused"),
        Err(err) => err.to_string(),
    }
}

#[test]
fn a_fresh_bake_is_added_as_the_next_version_with_its_recipe_recorded_and_cleared() {
    let p = reaching();
    assert!(refused(&p).contains("no bake"), "{}", refused(&p));
    let (bakes, done) = baked_once(&p);
    let review = done
        .review
        .clone()
        .expect("a done bake reads with its review");
    assert!(!review.lines.is_empty());
    let claim = claim(&bakes, &p, done.finished_ms).expect("claim");
    assert_eq!(claim.record.state, RecipeBakeState::Done);
    let staged = claim.staged.clone();
    assert!(staged.is_file());

    let version = accept(&p, &staged);
    assert_eq!(version, 2);
    let added = claim.added(p.root(), &p.config, version);
    assert_eq!(added.state, RecipeBakeState::Added);
    assert_eq!(added.added_version, Some(2));
    assert!(!staged.exists(), "the staged model is cleared");

    // The new version is recorded once, with the recipe and its version
    // that made it, and as Mixamo's.
    let result = done.result.expect("result");
    let uses = baked::of_asset(&p.dir, "elsheria").expect("bakes");
    assert_eq!(
        uses,
        vec![baked::RecipeBakeUse {
            recipe: p.recipe.id.clone(),
            recipe_version: result.recipe_version.clone(),
            base_clip: "still".into(),
            clip: result.clip.clone(),
            mixamo: true,
            version: 2,
        }]
    );
    assert!(crate::packs::licence::baked_mixamo(
        p.root(),
        &p.dir,
        "elsheria"
    ));

    // It reads as added and is never added twice, nor out of date.
    let read = record_of(p.root(), &p.config, &p.id, &p.recipe.id)
        .expect("read")
        .expect("record");
    assert_eq!(read.state, RecipeBakeState::Added);
    assert_eq!(read.stale, None);
    assert!(
        refused(&p).contains("added already, as v2"),
        "{}",
        refused(&p)
    );
}

#[test]
fn a_bake_whose_recipe_or_model_moved_on_is_refused_and_asks_for_a_new_bake() {
    let mut p = reaching();
    let (_bakes, _) = baked_once(&p);
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some([0.3, 1.2, 0.35]))],
    );
    let recipe = refused(&p);
    assert!(recipe.starts_with(OUT_OF_DATE), "{recipe}");
    assert!(recipe.contains(RECIPE_CHANGED), "{recipe}");

    let p = reaching();
    let (_bakes, done) = baked_once(&p);
    // Someone adds another version first.
    let other = p.root().join("other.glb");
    fs::write(&other, humanoid()).expect("other");
    accept(&p, &other);
    let model = refused(&p);
    assert!(model.starts_with(OUT_OF_DATE), "{model}");
    assert!(model.contains("v2"), "{model}");
    assert!(
        PathBuf::from(done.result.expect("result").file).is_file(),
        "kept"
    );
}

#[test]
fn a_version_that_came_in_without_its_mark_is_still_recorded_as_the_bake() {
    let p = reaching();
    let (bakes, done) = baked_once(&p);
    let claim = claim(&bakes, &p, done.finished_ms).expect("claim");
    // A file that lost its mark on the way in: the accept records nothing.
    let plain = p.root().join("plain.glb");
    fs::write(&plain, humanoid()).expect("plain");
    let version = accept(&p, &plain);
    assert!(
        baked::of_asset(&p.dir, "elsheria")
            .expect("read")
            .is_empty()
    );
    claim.added(p.root(), &p.config, version);
    let uses = baked::of_asset(&p.dir, "elsheria").expect("read");
    assert_eq!(uses.len(), 1);
    let result = done.result.expect("result");
    assert_eq!(
        (
            uses[0].recipe.as_str(),
            uses[0].recipe_version.as_str(),
            uses[0].mixamo,
            uses[0].version
        ),
        (
            p.recipe.id.as_str(),
            result.recipe_version.as_str(),
            true,
            version
        )
    );
}

#[test]
fn the_review_holds_each_number_to_its_limit() {
    let p = reaching();
    let prepared = prepare(p.root(), &p.config, &p.request()).expect("prepared");
    let work = p.root().join("work");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join(super::job::REPORT_FILE), super::tests::REPORT).expect("report");
    let run = super::result::read_run(&work).expect("run");
    let mut result = super::result::result_of(&run, &prepared, &work, 3.0);
    let review = review_of(&result);
    let line = |id: &str| {
        review
            .lines
            .iter()
            .find(|l| l.id == id)
            .unwrap_or_else(|| panic!("no {id}"))
            .clone()
    };
    // l-1 ends 1.2 cm off: within 2 cm. l-2 is 5 cm off and flips. l-3 was
    // never measured.
    assert_eq!(line("gap-l-1").status, RecipeReviewStatus::Pass);
    assert_eq!(line("gap-l-1").label, "Hand gap");
    assert_eq!(line("flip-l-1").status, RecipeReviewStatus::Pass);
    assert_eq!(line("gap-l-2").status, RecipeReviewStatus::Warn);
    assert_eq!(line("flip-l-2").status, RecipeReviewStatus::Warn);
    assert_eq!(line("flip-l-2").value, "flips at frame 20");
    assert_eq!(line("gap-l-3").status, RecipeReviewStatus::Warn);
    assert_eq!(line("gap-l-3").label, "Foot gap");
    assert_eq!(line("flip-l-3").label, "Knee");
    // 3.2 mm against the base's 1.0 + 5 passes; no feet planted slide.
    assert_eq!(line("stretch").status, RecipeReviewStatus::Pass);
    assert_eq!(line("stretch").max, Some(6.0));
    assert_eq!(line("feet").status, RecipeReviewStatus::Pass);
    assert_eq!(review.warns, 3);

    // A slide past the base's own plus 1.5 cm warns; at it, passes.
    result.base_slide_cm = 0.5;
    result.slide_cm = 2.0;
    assert_eq!(line_of(&result, "feet").status, RecipeReviewStatus::Pass);
    result.slide_cm = 2.1;
    assert_eq!(line_of(&result, "feet").status, RecipeReviewStatus::Warn);
    result.stretch_mm = Some(6.5);
    assert_eq!(line_of(&result, "stretch").status, RecipeReviewStatus::Warn);
    result.stretch_mm = None;
    result.stretch_note = Some("No skin to measure.".into());
    let none = line_of(&result, "stretch");
    assert_eq!(none.status, RecipeReviewStatus::NotMeasured);
    assert_eq!(none.note.as_deref(), Some("No skin to measure."));
}

fn line_of(result: &RecipeBakeResult, id: &str) -> RecipeReviewLine {
    review_of(result)
        .lines
        .into_iter()
        .find(|l| l.id == id)
        .expect("line")
}

#[test]
fn a_record_from_before_the_replaced_list_still_reads() {
    let (_bakes, done) = baked_once(&reaching());
    let mut json = serde_json::to_value(done.result.expect("result")).expect("json");
    let fields = json.as_object_mut().expect("object");
    fields.remove("replaced");
    fields.insert("replaces".into(), serde_json::json!("wave"));
    let read: RecipeBakeResult = serde_json::from_value(json).expect("old record");
    assert!(read.replaced.is_empty());
}

#[test]
fn an_add_holds_the_recipe_and_takes_only_the_bake_the_person_looked_at() {
    let p = reaching();
    let (bakes, _) = baked_once(&p);
    let done = settled(&bakes, &p);
    // Another bake than the one looked at: refused, nothing held.
    let other = Some(done.finished_ms.expect("finished") - 1.0);
    let Err(err) = claim(&bakes, &p, other) else {
        panic!("not refused");
    };
    assert!(err.to_string().starts_with(BAKE_CHANGED), "{err}");

    let held = claim(&bakes, &p, done.finished_ms).expect("claim");
    // While it is held, a second add and a new bake are both refused.
    let Err(twice) = claim(&bakes, &p, done.finished_ms) else {
        panic!("added twice");
    };
    assert!(twice.to_string().contains("being added already"), "{twice}");
    let start = bakes.start(
        p.root(),
        &p.config,
        Toolbox::default(),
        &p.request(),
        RecipeBakeStarter::Person,
    );
    let Err(baking) = start else {
        panic!("baked during an add");
    };
    assert!(baking.to_string().contains("being added"), "{baking}");

    // Let go without adding: the recipe is free again.
    drop(held);
    claim(&bakes, &p, done.finished_ms).expect("free again");
}

#[test]
fn a_finished_add_is_saved_even_when_the_bakes_record_cannot_be_written() {
    let p = reaching();
    let (bakes, done) = baked_once(&p);
    let claim = claim(&bakes, &p, done.finished_ms).expect("claim");
    let version = accept(&p, &claim.staged.clone());
    // recipe-bakes.json is a newer format this build cannot write over.
    fs::write(
        p.dir.join(baked::BAKES_FILE),
        r#"{"version": 99, "assets": {}}"#,
    )
    .expect("newer");
    let added = claim.added(p.root(), &p.config, version);
    assert_eq!(added.state, RecipeBakeState::Added);
    let read = record_of(p.root(), &p.config, &p.id, &p.recipe.id)
        .expect("read")
        .expect("record");
    assert_eq!(read.added_version, Some(version));
    assert!(refused(&p).contains("added already"), "{}", refused(&p));
}

#[test]
fn saving_the_recipe_to_the_library_leaves_its_bake_up_to_date_and_ready_to_add() {
    use crate::animations::recipes::library::{RecipeSave, plan_save, save_back};
    let mut p = reaching();
    let (_bakes, done) = baked_once(&p);
    let before = recipes::version(&p.dir, &p.recipe.id).expect("version");

    // The save writes the library link into the project's recipe file.
    let plan = plan_save(p.root(), &p.config.layout, &p.id, &p.recipe.id, None).expect("plan");
    save_back(
        p.root(),
        &p.config.layout,
        &RecipeSave {
            project: p.id.clone(),
            recipe: p.recipe.id.clone(),
            entry: None,
            expected: None,
            recipe_expected: Some(plan.recipe_version),
        },
    )
    .expect("save");
    let after = recipes::version(&p.dir, &p.recipe.id).expect("version");
    assert_ne!(before, after, "the file's own version moved with the link");

    let read = record_of(p.root(), &p.config, &p.id, &p.recipe.id)
        .expect("read")
        .expect("record");
    assert_eq!(read.stale, None, "the motion did not change");
    ready_to_add(p.root(), &p.config, &p.id, &p.recipe.id).expect("still ready");

    // A real change still makes it stale.
    assert_eq!(done.state, RecipeBakeState::Done);
    p.change(
        vec![reach("l-1", "rightHand", "b-1")],
        vec![point("b-1", Some([0.3, 1.2, 0.35]))],
    );
    let why = refused(&p);
    assert!(why.contains(RECIPE_CHANGED), "{why}");
}

#[test]
fn a_bake_record_from_before_the_motion_version_reads_by_the_files_version() {
    let p = reaching();
    let (_bakes, done) = baked_once(&p);
    let mut record = done;
    let result = record.result.as_mut().expect("result");
    result.motion_version = String::new();
    assert_eq!(stale_of(p.root(), &p.config, &record), None);
    record.result.as_mut().expect("result").recipe_version = "older".into();
    assert_eq!(
        stale_of(p.root(), &p.config, &record).as_deref(),
        Some(RECIPE_CHANGED)
    );
}
