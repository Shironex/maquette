//! The watcher looks at one Downloads folder for every project. Two
//! projects waiting for an asset of the same id, under the same job id,
//! each get the file saved under their own name and never the other's.

use super::*;
use crate::providers::tests::ids::{request_in, second_project};

fn by_hand_in(rig: &Rig, project: &str) -> ProviderJob {
    rig.jobs
        .send(
            &request_in(&rig.world, project, Provider::ByHand),
            false,
            false,
        )
        .expect("send")
}

fn state(rig: &Rig, job: &ProviderJob) -> JobState {
    rig.jobs.get(&job.project, &job.id).expect("job").state
}

#[test]
fn each_projects_download_lands_in_its_own_send() {
    let rig = rig();
    let other = second_project(&rig.world);
    let mine = by_hand_in(&rig, &rig.world.project);
    let theirs = by_hand_in(&rig, &other);
    assert_eq!(mine.id, theirs.id, "one id, two sends");
    let mut waits: Vec<(String, String)> = rig
        .arrivals
        .status()
        .pending
        .into_iter()
        .map(|p| (p.project, p.expected_name))
        .collect();
    waits.sort();
    assert_eq!(
        waits,
        [
            (
                other.clone(),
                "maq_lantern-bay_stone-lantern_v1_tripo".to_string()
            ),
            (
                rig.world.project.clone(),
                "maq_moon-island_stone-lantern_v1_tripo".to_string()
            ),
        ]
    );

    let file = rig.download(
        "maq_lantern-bay_stone-lantern_v1_tripo.glb",
        &fixture::character(false),
    );
    let arrival = rig.arrivals.take(&file).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.project.as_deref(), Some(other.as_str()));
    assert_eq!(arrival.job.as_deref(), Some(theirs.id.as_str()));
    assert_eq!(state(&rig, &theirs), JobState::Inbox);
    assert_eq!(
        state(&rig, &mine),
        JobState::Waiting,
        "not the other project's file"
    );

    let file = rig.download(
        "maq_moon-island_stone-lantern_v1_tripo (1).glb",
        &fixture::clean_prop(),
    );
    let arrival = rig.arrivals.take(&file).expect("taken");
    assert_eq!(arrival.state, ArrivalState::Imported, "{arrival:?}");
    assert_eq!(arrival.project.as_deref(), Some(rig.world.project.as_str()));
    assert_eq!(state(&rig, &mine), JobState::Inbox);

    // Each model is in the inbox for its own project.
    let mut items = inbox::list(&rig.world.root, &rig.world.config).expect("inbox");
    rig.jobs.link_inbox(&mut items);
    assert_eq!(items.len(), 2);
    for (job, project) in [(&mine, &rig.world.project), (&theirs, &other)] {
        let file = rig.jobs.get(project, &job.id).and_then(|j| j.inbox_file);
        let item = items
            .iter()
            .find(|i| Some(&i.id) == file.as_ref())
            .expect("item");
        assert_eq!(item.link.as_ref().map(|l| &l.project), Some(project));
    }
    // And each settled its own line in the ledger.
    let ledger = rig.jobs.ledger().expect("ledger");
    let mut settled: Vec<(&str, SpendOutcome)> = ledger
        .spends
        .iter()
        .map(|s| (s.project.as_str(), s.outcome))
        .collect();
    settled.sort_by(|a, b| a.0.cmp(b.0));
    assert_eq!(
        settled,
        [
            (other.as_str(), SpendOutcome::Succeeded),
            (rig.world.project.as_str(), SpendOutcome::Succeeded),
        ]
    );
}

#[test]
fn a_download_named_for_another_projects_send_is_asked_about_never_taken() {
    let rig = rig();
    let other = second_project(&rig.world);
    // Only the second project waits; the file says it is the first's.
    let theirs = by_hand_in(&rig, &other);
    let file = rig.download(
        "maq_moon-island_stone-lantern_v1_tripo.glb",
        &fixture::character(false),
    );
    let arrival = rig.arrivals.take(&file).expect("seen");
    assert_eq!(arrival.state, ArrivalState::Unmatched, "{arrival:?}");
    assert_eq!(
        arrival.reason,
        "Named for stone-lantern v1 on Tripo in moon-island, which is not waiting for a model"
    );
    assert_eq!(arrival.job, None, "not even suggested");
    assert_eq!(arrival.suggestions.len(), 1);
    assert_eq!(
        arrival.suggestions[0].name, 0.0,
        "its name fits no other send"
    );
    assert_eq!(state(&rig, &theirs), JobState::Waiting);

    // The bare job id is both projects' once the first waits too.
    let mine = by_hand_in(&rig, &rig.world.project);
    let bare = rig.download("stone-lantern-1.glb", &fixture::clean_prop());
    let asked = rig.arrivals.take(&bare).expect("seen");
    assert_eq!(asked.state, ArrivalState::Unmatched, "{asked:?}");
    assert_eq!(
        asked.reason,
        "2 sends named stone-lantern-1 are waiting, in lantern-bay and moon-island"
    );
    assert_eq!(state(&rig, &mine), JobState::Waiting);
    assert_eq!(state(&rig, &theirs), JobState::Waiting);

    // A person says which project it is for: that send takes it, alone.
    let done = rig
        .arrivals
        .attach(&asked.id, &other, &rig.world.asset)
        .expect("attach");
    assert_eq!(done.state, ArrivalState::Attached);
    assert_eq!(done.project.as_deref(), Some(other.as_str()));
    assert_eq!(state(&rig, &theirs), JobState::Inbox);
    assert_eq!(state(&rig, &mine), JobState::Waiting);
}
