//! A real MCP client reads the starter recipes built into the app from
//! list_library_recipes and get_library_recipe, uses them in a project
//! (a found clip, a missing one and the rest pose) and makes a recipe on
//! the rest pose; none of it writes to the recipe library. Everything
//! happens in a temp workspace; no Blender runs.

use maquette_core::animations::recipes::library;
use serde_json::json;

use super::animations::fixture;
use super::start::serve;
use super::support::{call, call_error};

const PROJECT: &str = "frostveil";

#[tokio::test(flavor = "multi_thread")]
async fn agents_read_and_use_starters_and_make_a_rest_pose_recipe() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, host) = serve(fixture(dir.path())).await;
    let (root, _) = host.session.workspace().expect("ws");

    let listed = call(&client, "list_library_recipes", json!({})).await;
    assert_eq!(listed["recipes"], json!([]));
    let starters = listed["starters"].as_array().expect("starters");
    assert_eq!(starters.len(), 5, "{listed}");
    assert!(starters.iter().all(|s| s["starter"] == true));
    let wave = &starters[0];
    assert_eq!(wave["id"], "starter:wave-hello");
    assert_eq!(wave["base"]["rest"], true);
    assert_eq!(wave["base"]["library"], "rest");
    assert!(
        !library::dir(&root).exists(),
        "listing starters writes nothing"
    );

    let read = call(
        &client,
        "get_library_recipe",
        json!({ "entry": "starter:point-while-walking" }),
    )
    .await;
    assert_eq!(read["base"]["names"], json!(["Walking", "Walk"]), "{read}");
    assert_eq!(read["base"]["library"], "missing");
    assert_eq!(read["changes"][0]["kind"], "reach");
    assert!(!read.to_string().contains("\"hash\""), "{read}");

    let used = call(
        &client,
        "use_recipe",
        json!({ "project": PROJECT, "entry": "starter:wave-hello" }),
    )
    .await;
    assert_eq!(used["id"], "wave-hello", "{used}");
    assert_eq!(used["base"]["rest"], true);
    assert_eq!(used["base"]["library"], "rest");
    assert_eq!(used["mixamo"], false);

    let walking = call(
        &client,
        "use_recipe",
        json!({ "project": PROJECT, "entry": "starter:point-while-walking" }),
    )
    .await;
    assert_eq!(walking["base"]["library"], "missing", "{walking}");
    assert_eq!(walking["base"]["hash"], "");
    assert!(
        walking["notes"].to_string().contains("Walking"),
        "{walking}"
    );

    let revised = call_error(
        &client,
        "use_recipe",
        json!({ "project": PROJECT, "entry": "starter:lean-in", "revision": 1 }),
    )
    .await;
    assert!(revised.contains("no revisions"), "{revised}");

    let rest = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "restPose": true, "name": "Stand and look" }),
    )
    .await;
    assert_eq!(rest["base"]["name"], "Rest pose", "{rest}");
    assert_eq!(rest["base"]["frames"], 60);
    let both = call_error(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "restPose": true, "clip": "standing-dodge-left" }),
    )
    .await;
    assert!(both.contains("one of the three"), "{both}");
    let neither = call_error(&client, "create_recipe", json!({ "project": PROJECT })).await;
    assert!(neither.contains("restPose"), "{neither}");

    assert!(
        !library::dir(&root).exists(),
        "using starters never fills the library"
    );
    client.cancel().await.expect("close");
    server.stop();
}
