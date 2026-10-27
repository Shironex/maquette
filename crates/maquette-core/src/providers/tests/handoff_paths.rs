//! By hand folders in `providers.json`: saved relative to the workspace so
//! the file reads on every computer it syncs to, and older absolute paths
//! read back where they are on this computer.

use super::*;

fn saved_folder(world: &World) -> Value {
    let path = world.project_dir().join(jobs::JOBS_FILE);
    let file: Value = serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json");
    file["jobs"][0]["job"]["handoff"].clone()
}

/// Rewrites the first saved job's folder as an older Maquette wrote it.
fn save_folder_as(world: &World, folder: &str) {
    let path = world.project_dir().join(jobs::JOBS_FILE);
    let mut file: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    file["jobs"][0]["job"]["handoff"] = json!(folder);
    fs::write(&path, file.to_string()).expect("write");
}

fn by_hand(world: &World, server: &FakeServer) -> (ProviderJobs, ProviderJob) {
    let jobs = world.jobs(server);
    let request = world.request(Provider::ByHand, PresetVariant::Prop, &["front.png"]);
    let job = jobs.send(&request, false, false).expect("send");
    (jobs, job)
}

#[test]
fn a_handoff_folder_is_saved_relative_to_the_workspace() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let (jobs, job) = by_hand(&world, &server);
    let folder = world.project_dir().join("handoff").join(&job.id);
    assert_eq!(job.handoff, Some(folder.display().to_string()));
    assert_eq!(
        saved_folder(&world),
        json!(format!("projects/moon-island/handoff/{}", job.id))
    );
    jobs.close();
    let again = world.jobs(&server);
    let reread = again.get(&job.id).expect("job");
    assert_eq!(reread.handoff, Some(folder.display().to_string()));
}

#[test]
fn an_older_absolute_folder_inside_the_workspace_is_converted() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let (jobs, job) = by_hand(&world, &server);
    jobs.close();
    let folder = world.project_dir().join("handoff").join(&job.id);
    save_folder_as(&world, &folder.display().to_string());

    let again = world.jobs(&server);
    assert_eq!(
        again.get(&job.id).expect("job").handoff,
        Some(folder.display().to_string())
    );
    assert_eq!(
        saved_folder(&world),
        json!(format!("projects/moon-island/handoff/{}", job.id)),
        "written back in the new form"
    );
}

#[test]
fn another_computers_folder_reads_as_this_ones_and_an_outside_one_stays() {
    let world = World::new();
    let server = FakeServer::start(|_, _| Reply::json(404, &json!({})));
    let (jobs, job) = by_hand(&world, &server);
    jobs.close();
    let other = format!(
        "/Volumes/other-computer/Maquette/projects/moon-island/handoff/{}",
        job.id
    );
    save_folder_as(&world, &other);
    let again = world.jobs(&server);
    let own = world.project_dir().join("handoff").join(&job.id);
    assert_eq!(
        again.get(&job.id).expect("job").handoff,
        Some(own.display().to_string())
    );
    again.close();

    // A folder outside the workspace that is there is left where it is.
    let outside = tempfile::tempdir().expect("outside");
    let kept = outside.path().display().to_string();
    save_folder_as(&world, &kept);
    let third = world.jobs(&server);
    assert_eq!(third.get(&job.id).expect("job").handoff, Some(kept.clone()));
    third.close();
    assert_eq!(saved_folder(&world), json!(kept));
}
