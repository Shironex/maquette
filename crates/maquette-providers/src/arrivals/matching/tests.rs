//! Tests for deciding which waiting send a download is for.

use super::*;
use crate::jobs::{CancelMode, JobState, Provider};
use crate::presets::{PresetVariant, ProviderId};

pub(in crate::arrivals) const T: f64 = 1_800_000_000_000.0;
const MIN: f64 = 60_000.0;

pub(in crate::arrivals) fn waiting(asset: &str, name: &str, n: u32, created: f64) -> ProviderJob {
    ProviderJob {
        id: format!("{asset}-{n}"),
        project: "moon".into(),
        asset: asset.into(),
        asset_name: name.into(),
        provider: Provider::ByHand,
        service: ProviderId::Tripo,
        variant: PresetVariant::Prop,
        images: vec![],
        views: Vec::new(),
        tripo: None,
        meshy: None,
        state: JobState::Waiting,
        phase: None,
        progress: 0.0,
        remote_id: None,
        credits: None,
        created_at_ms: created,
        started_at_ms: None,
        finished_at_ms: None,
        message: None,
        inbox_file: None,
        handoff: None,
        card: None,
        version: None,
        via_agent: false,
        cancel: CancelMode::StopWaiting,
        batch: None,
        prompt: None,
        redo_of: None,
        estimate: None,
        split: None,
        studio: None,
        retexture: None,
        denied: false,
    }
}

/// A send of `project` handed out the way one is now: its card carries the
/// export name with the project in it.
pub(in crate::arrivals) fn handed(
    project: &str,
    asset: &str,
    name: &str,
    n: u32,
    created: f64,
) -> ProviderJob {
    let mut job = waiting(asset, name, n, created);
    job.project = project.into();
    let export = naming::export_stem(project, &job.id, asset, ProviderId::Tripo);
    job.card = Some(card(&export));
    job
}

/// A settings card that asks for the export name `export`.
fn card(export: &str) -> crate::handoff::HandoffCard {
    crate::handoff::HandoffCard {
        service: ProviderId::Tripo,
        site: String::new(),
        mode: "Image to 3D".into(),
        rows: Vec::new(),
        images: Vec::new(),
        return_name: format!("{export}.glb"),
        export_name: export.into(),
        credits: None,
        credits_note: None,
        prompt: None,
    }
}

/// The job a verdict takes the file for by itself.
fn id(v: &Verdict) -> Option<&str> {
    match v {
        Verdict::Exact { job, .. } => Some(&job.id),
        Verdict::Close { .. } | Verdict::Ask { .. } => None,
    }
}

/// The project and id of the job a verdict takes the file for by itself.
fn taken(v: &Verdict) -> Option<(&str, &str)> {
    match v {
        Verdict::Exact { job, .. } => Some((&job.project, &job.id)),
        Verdict::Close { .. } | Verdict::Ask { .. } => None,
    }
}

/// The job a verdict only suggests.
fn suggested(v: &Verdict) -> Option<&str> {
    match v {
        Verdict::Close { job, .. } => Some(&job.id),
        Verdict::Exact { .. } | Verdict::Ask { .. } => None,
    }
}

fn asked(v: &Verdict) -> &str {
    match v {
        Verdict::Ask { reason } => reason,
        Verdict::Exact { .. } | Verdict::Close { .. } => panic!("not asked: {v:?}"),
    }
}

#[test]
fn two_projects_waiting_for_the_same_asset_each_get_their_own_file() {
    let jobs = [
        handed("moon", "desk-lamp", "Desk lamp", 1, T),
        handed("bay", "desk-lamp", "Desk lamp", 1, T + MIN),
    ];
    assert_eq!(jobs[0].id, jobs[1].id, "both are the first send");
    let v = decide(
        "maq_moon_desk-lamp_v1_tripo.glb",
        None,
        T + 2.0 * MIN,
        &jobs,
    );
    assert_eq!(taken(&v), Some(("moon", "desk-lamp-1")));
    let v = decide(
        "MAQ_Bay_Desk-Lamp_V1_Tripo (1).GLB",
        None,
        T + 2.0 * MIN,
        &jobs,
    );
    assert_eq!(taken(&v), Some(("bay", "desk-lamp-1")));
    // Inside a zip, a day later.
    let v = decide(
        "download.zip",
        Some("model/maq_bay_desk-lamp_v1_tripo.glb"),
        T + 26.0 * 60.0 * MIN,
        &jobs,
    );
    assert_eq!(taken(&v), Some(("bay", "desk-lamp-1")));
    // The older name, with no project, is neither send's.
    let v = decide("maq_desk-lamp_v1_tripo.glb", None, T + 2.0 * MIN, &jobs);
    assert_eq!(
        asked(&v),
        "Named for desk-lamp v1 on Tripo, which is not waiting for a model"
    );
    // Nor is a third project's.
    let v = decide(
        "maq_cove_desk-lamp_v1_tripo.glb",
        None,
        T + 2.0 * MIN,
        &jobs,
    );
    assert_eq!(
        asked(&v),
        "Named for desk-lamp v1 on Tripo in cove, which is not waiting for a model"
    );
    // The bare id is both sends': asked, never a pick between projects.
    let v = decide("desk-lamp-1.glb", None, T + 2.0 * MIN, &jobs);
    assert_eq!(
        asked(&v),
        "2 sends named desk-lamp-1 are waiting, in bay and moon"
    );
}

#[test]
fn a_file_saved_under_an_older_name_still_matches_its_waiting_send() {
    // Handed out before the project was part of the name: the card says so.
    let mut older = waiting("desk-lamp", "Desk lamp", 5, T);
    older.project = "bay".into();
    older.card = Some(card("maq_desk-lamp_v5_tripo"));
    // A card from before export names waits for the same form, from its id.
    let mut oldest = waiting("crate", "Crate", 2, T);
    oldest.project = "bay".into();
    let jobs = [
        older,
        oldest,
        handed("moon", "desk-lamp", "Desk lamp", 5, T + MIN),
        handed("moon", "crate", "Crate", 2, T + MIN),
    ];
    let v = decide("maq_desk-lamp_v5_tripo (1).glb", None, T + 2.0 * MIN, &jobs);
    assert_eq!(taken(&v), Some(("bay", "desk-lamp-5")));
    let v = decide("maq_crate_v2_tripo.zip", None, T + 2.0 * MIN, &jobs);
    assert_eq!(taken(&v), Some(("bay", "crate-2")));
    // The same sends of the other project wait for their own names.
    let v = decide(
        "maq_moon_desk-lamp_v5_tripo.glb",
        None,
        T + 2.0 * MIN,
        &jobs,
    );
    assert_eq!(taken(&v), Some(("moon", "desk-lamp-5")));
    let v = decide("maq_moon_crate_v2_tripo.glb", None, T + 2.0 * MIN, &jobs);
    assert_eq!(taken(&v), Some(("moon", "crate-2")));
}

#[test]
fn a_name_two_sends_wait_for_is_asked_about() {
    // Two records that were handed the same name (a copied project folder).
    let mut copy = handed("moon", "desk-lamp", "Desk lamp", 1, T);
    copy.project = "moon copy".into();
    let jobs = [handed("moon", "desk-lamp", "Desk lamp", 1, T), copy];
    let v = decide("maq_moon_desk-lamp_v1_tripo.glb", None, T + MIN, &jobs);
    assert_eq!(asked(&v), "2 sends wait for a file of that name");
}

#[test]
fn a_file_named_for_another_send_fits_no_send_by_its_name() {
    let bay = handed("bay", "desk-lamp", "Desk lamp", 1, T);
    // Named for the same asset in another project, and for another number.
    for other in [
        "maq_moon_desk-lamp_v1_tripo.glb",
        "maq_bay_desk-lamp_v2_tripo.glb",
        "maq_bay_desk-lamp_v1_meshy.glb",
        "maq_desk-lamp_v1_tripo.glb",
    ] {
        assert_eq!(name_fit(other, None, &bay), 0.0, "{other}");
        let v = decide(other, None, T + MIN, std::slice::from_ref(&bay));
        assert!(
            matches!(v, Verdict::Ask { .. }),
            "{other}: not even the only send waiting takes it: {v:?}"
        );
    }
    assert_eq!(
        name_fit("maq_bay_desk-lamp_v1_tripo (2).glb", None, &bay),
        1.0
    );
    assert_eq!(name_fit("Desk_Lamp_textured.glb", None, &bay), 1.0);
}

#[test]
fn the_export_name_wins_over_time_and_other_names() {
    let jobs = [
        waiting("desk-lamp", "Desk lamp", 2, T),
        waiting("crate", "Crate", 1, T + MIN),
    ];
    let v = decide("maq_desk-lamp_v2_tripo (1).glb", None, T + 2.0 * MIN, &jobs);
    assert_eq!(id(&v), Some("desk-lamp-2"));
    // Even a day later, and inside a zip.
    let v = decide(
        "download.zip",
        Some("maq_crate_v1_tripo.glb"),
        T + 30.0 * 60.0 * MIN,
        &jobs,
    );
    assert_eq!(id(&v), Some("crate-1"));
    // Named for a send that is not waiting: asked, never guessed.
    let v = decide("maq_desk-lamp_v1_tripo.glb", None, T + 2.0 * MIN, &jobs);
    assert!(matches!(v, Verdict::Ask { .. }), "{v:?}");
    // Named for the right send on the other provider: asked too.
    let v = decide("maq_crate_v1_meshy.glb", None, T + 2.0 * MIN, &jobs);
    assert!(matches!(v, Verdict::Ask { .. }), "{v:?}");
    // The old handoff name still works.
    let v = decide("Crate-1.glb", None, T + 2.0 * MIN, &jobs);
    assert_eq!(id(&v), Some("crate-1"));
}

#[test]
fn two_sends_waiting_with_no_name_means_asking() {
    let jobs = [
        waiting("desk-lamp", "Desk lamp", 1, T),
        waiting("crate", "Crate", 1, T + 8.0 * 60.0 * MIN),
    ];
    let v = decide("model (3).glb", None, T + 8.0 * 60.0 * MIN + MIN, &jobs);
    let Verdict::Ask { reason } = v else {
        panic!("guessed {v:?}");
    };
    assert_eq!(reason, "2 sends are waiting");
}

#[test]
fn without_a_name_the_only_send_is_only_suggested_however_late() {
    let jobs = [waiting("desk-lamp", "Desk lamp", 1, T)];
    let v = decide("model (3).glb", None, T + 10.0 * MIN, &jobs);
    assert_eq!(suggested(&v), Some("desk-lamp-1"));
    assert_eq!(id(&v), None, "never taken by itself");
    // Two days later it is still suggested, never refused.
    let v = decide("model (3).glb", None, T + 48.0 * 60.0 * MIN, &jobs);
    assert_eq!(suggested(&v), Some("desk-lamp-1"));
    // Before the handoff (beyond the clock slack): asked.
    assert!(matches!(
        decide("model (3).glb", None, T - 10.0 * MIN, &jobs),
        Verdict::Ask { .. }
    ));
}

#[test]
fn a_close_name_is_suggested_a_day_after_its_handoff() {
    let jobs = [
        waiting("stone-ruin", "Stone ruin", 1, T),
        waiting("wooden-log", "Wooden log", 1, T + 60.0 * MIN),
    ];
    let v = decide(
        "stone ruins 3d model.glb",
        None,
        T + 26.0 * 60.0 * MIN,
        &jobs,
    );
    assert_eq!(suggested(&v), Some("stone-ruin-1"));
}

#[test]
fn several_waiting_sends_need_a_clear_name_or_a_question() {
    let jobs = [
        waiting("desk-lamp", "Desk lamp", 1, T),
        waiting("brush-cup", "Brush cup", 1, T),
    ];
    let v = decide("tripo_desk_lamp_8c1f.glb", None, T + MIN, &jobs);
    assert_eq!(suggested(&v), Some("desk-lamp-1"));
    let v = decide("model.glb", None, T + MIN, &jobs);
    assert_eq!(
        v,
        Verdict::Ask {
            reason: "2 sends are waiting".into()
        }
    );
    // A name that fits both equally picks neither.
    let jobs = [
        waiting("lamp-a", "Lamp", 1, T),
        waiting("lamp-b", "Lamp", 1, T),
    ];
    assert!(matches!(
        decide("lamp.glb", None, T + MIN, &jobs),
        Verdict::Ask { .. }
    ));
}

#[test]
fn only_the_export_name_places_a_file_and_a_close_name_waits_for_a_look() {
    let jobs = [
        waiting("stone-ruin", "Stone ruin", 1, T),
        waiting("wooden-log", "Wooden log", 1, T),
        waiting("lantern-post", "Lantern post", 1, T),
    ];
    // Exact: the export name, and the older handoff name.
    let v = decide("maq_stone-ruin_v1_tripo.glb", None, T + MIN, &jobs);
    assert_eq!(id(&v), Some("stone-ruin-1"));
    let v = decide("Wooden-Log-1.glb", None, T + MIN, &jobs);
    assert_eq!(id(&v), Some("wooden-log-1"));
    // Fuzzy: the site's own batch names are a suggestion, with why.
    let v = decide("stone ruins 3d model.glb", None, T + MIN, &jobs);
    assert_eq!(
        v,
        Verdict::Close {
            job: JobKey::new("moon", "stone-ruin-1"),
            reason: "Name close to Stone ruin".into()
        }
    );
    let v = decide("wooden log 3d model.glb", None, T + MIN, &jobs);
    assert_eq!(suggested(&v), Some("wooden-log-1"));
    // None: a name that fits no send is asked about.
    let v = decide("tree stump 3d model.glb", None, T + MIN, &jobs);
    assert_eq!(
        v,
        Verdict::Ask {
            reason: "3 sends are waiting".into()
        }
    );
}
