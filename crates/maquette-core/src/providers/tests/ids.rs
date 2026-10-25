//! Job ids across projects: two projects sending an asset of the same id
//! get different job ids, and saved jobs from before that rule are given
//! unique ones when the workspace opens.

use super::*;

/// A second project with an asset of the same id as the world's.
fn second_project(world: &World) -> String {
    let manifest = project::create(
        &world.root,
        &world.config.layout,
        NewProject {
            name: "Lantern Bay".into(),
            template: ProjectTemplate::FloatingDiorama,
            site_repo: None,
            source_folder: None,
        },
    )
    .expect("project");
    project::modify(&world.root, &world.config.layout, &manifest.id, |m| {
        let asset = assets::add(
            m,
            NewAsset {
                name: "Stone lantern".into(),
                kind: AssetKind::Prop,
                stage: Stage::Refs,
            },
        )?;
        assert_eq!(asset.id, world.asset, "the same asset id in both");
        Ok(())
    })
    .expect("asset");
    let refs = project::dir(&world.root, &world.config.layout, &manifest.id)
        .expect("dir")
        .join("refs");
    fs::create_dir_all(&refs).expect("refs");
    fs::write(refs.join("front.png"), b"\x89PNG\r\n\x1a\nfake").expect("image");
    manifest.id
}

fn request_in(world: &World, project: &str) -> SendRequest {
    SendRequest {
        project: project.to_string(),
        ..world.request(Provider::Meshy, PresetVariant::Prop, &["front.png"])
    }
}

fn jobs_file(world: &World, project: &str) -> PathBuf {
    project::dir(&world.root, &world.config.layout, project)
        .expect("dir")
        .join(jobs::JOBS_FILE)
}

#[test]
fn the_same_asset_in_two_projects_gets_two_job_ids() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let jobs = world.jobs(&server);

    let first = jobs
        .send(&request_in(&world, &world.project), false, false)
        .expect("first");
    assert_eq!(finished(&jobs, &first.id).state, JobState::Inbox);
    let second = jobs
        .send(&request_in(&world, &other), true, true)
        .expect("second");

    assert_ne!(first.id, second.id);
    assert_eq!(second.project, other, "the send answers with its own job");
    assert_eq!(second.state, JobState::Approval);
    assert_eq!(jobs.get(&first.id).expect("first").project, world.project);
    assert_eq!(jobs.get(&second.id).expect("second").project, other);

    let approved = jobs.approve(&second.id).expect("approve");
    assert_eq!(approved.project, other);
    assert_eq!(finished(&jobs, &second.id).state, JobState::Inbox);
    assert_eq!(
        jobs.get(&first.id).expect("first").state,
        JobState::Inbox,
        "the other project's job is left alone"
    );
    let ledger = settled(&jobs, 2);
    let mut keys: Vec<(String, String)> = ledger
        .spends
        .iter()
        .map(|s| (s.project.clone(), s.job.clone()))
        .collect();
    keys.sort();
    let mut want = vec![
        (world.project.clone(), first.id.clone()),
        (other.clone(), second.id.clone()),
    ];
    want.sort();
    assert_eq!(keys, want, "one ledger entry per send");
}

#[test]
fn saved_jobs_sharing_an_id_get_unique_ids_when_the_workspace_opens() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let first_id = {
        let jobs = world.jobs(&server);
        let first = jobs
            .send(&request_in(&world, &world.project), false, false)
            .expect("first");
        finished(&jobs, &first.id);
        jobs.send(&request_in(&world, &other), true, true)
            .expect("second");
        jobs.close();
        first.id
    };

    // Saved by an older Maquette: the second project's job and a redo of
    // it both use the first project's id.
    let path = jobs_file(&world, &other);
    let mut file: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    let mut redo = file["jobs"][0].clone();
    file["jobs"][0]["job"]["id"] = json!(first_id);
    redo["job"]["id"] = json!(format!("{}-redo", world.asset));
    redo["job"]["redoOf"] = json!(first_id);
    redo["job"]["state"] = json!("failed");
    file["jobs"].as_array_mut().expect("jobs").push(redo);
    fs::write(&path, file.to_string()).expect("write");

    let jobs = world.jobs(&server);
    let mine = jobs.list(Some(&world.project));
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].id, first_id, "the older job keeps its id");
    let theirs = jobs.list(Some(&other));
    let renamed = theirs
        .iter()
        .find(|j| j.redo_of.is_none())
        .expect("renamed job");
    assert_ne!(renamed.id, first_id);
    assert_eq!(jobs.get(&renamed.id).expect("by id").project, other);
    assert_eq!(jobs.get(&first_id).expect("by id").project, world.project);
    let follower = theirs.iter().find(|j| j.redo_of.is_some()).expect("redo");
    assert_eq!(follower.redo_of.as_deref(), Some(renamed.id.as_str()));

    let saved = fs::read_to_string(&path).expect("read");
    assert!(saved.contains(&format!("\"{}\"", renamed.id)), "saved");
}

#[test]
fn an_inbox_model_links_to_the_job_that_saved_it_even_under_a_shared_name() {
    let world = World::new();
    let other = second_project(&world);
    let (server, _) = meshy_server(false);
    let first = {
        let jobs = world.jobs(&server);
        let first = jobs
            .send(&request_in(&world, &world.project), false, false)
            .expect("first");
        let first = finished(&jobs, &first.id);
        let second = jobs
            .send(&request_in(&world, &other), false, false)
            .expect("second");
        finished(&jobs, &second.id);
        jobs.close();
        first
    };

    // How an older Maquette left it: both jobs `<asset>-1`, the second
    // model saved as `<asset>-1-2.glb`, a name that also reads as the first
    // job's.
    let shared = format!("{}-2.glb", first.id);
    let inbox = world.inbox();
    let second_file = fs::read_dir(&inbox)
        .expect("inbox")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.ends_with(".glb") && Some(n.as_str()) != first.inbox_file.as_deref())
        .expect("second model");
    fs::rename(inbox.join(&second_file), inbox.join(&shared)).expect("rename");
    let path = jobs_file(&world, &other);
    let mut file: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
    file["jobs"][0]["job"]["id"] = json!(first.id);
    file["jobs"][0]["job"]["inboxFile"] = json!(shared);
    fs::write(&path, file.to_string()).expect("write");

    let jobs = world.jobs(&server);
    let second = jobs.list(Some(&other)).remove(0);
    assert_ne!(second.id, first.id);
    let mut items = inbox::list(&world.root, &world.config).expect("inbox");
    assert_eq!(items.len(), 2);
    jobs.link_inbox(&mut items);
    for item in &items {
        let link = item.link.as_ref().expect("linked");
        let want = if item.id == shared { &second } else { &first };
        assert_eq!(link.job, want.id, "{}", item.id);
        assert_eq!(link.project, want.project, "{}", item.id);
    }
}
