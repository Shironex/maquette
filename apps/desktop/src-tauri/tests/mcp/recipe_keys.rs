//! A real MCP client and pose keys: set_recipe_keys adds a pose key change
//! and keys on it, moves and removes them by bone and frame, refuses a
//! stale version, and answers the keyed turns past the joint limits;
//! get_recipe_pose reads the blended turns at a frame. The recipe stays
//! bakeable. Nothing runs Blender.

use serde_json::{Value, json};

use super::animations::fixture;
use super::start::serve;
use super::support::{call, raw_call, text};

const PROJECT: &str = "frostveil";

async fn refused(client: &super::support::Client, tool: &str, args: Value) -> Value {
    let result = raw_call(client, tool, args).await;
    assert_eq!(result.is_error, Some(true), "{tool} should fail");
    serde_json::from_str(&text(&result)).expect("json")
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_key_bones_move_and_remove_keys_and_read_the_pose() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, _host) = serve(fixture(dir.path())).await;
    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left" }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();

    // A new pose key change: two keys on the chest, one past its twist limit.
    let keyed = call(
        &client,
        "set_recipe_keys",
        json!({
            "project": PROJECT, "recipe": id, "expected": made["version"],
            "set": [
                { "frame": 4, "pose": { "chest": [0.0, 10.0, 0.0], "head": [0.0, 0.0, 5.0] } },
                { "frame": 20, "ease": "out", "pose": { "chest": [0.0, 52.0, 0.0] } },
            ],
        }),
    )
    .await;
    let layer = keyed["layer"]["id"].as_str().expect("layer id").to_string();
    assert_eq!(keyed["layer"]["kind"], "pose", "{keyed}");
    assert_eq!(keyed["layer"]["mode"], "add", "{keyed}");
    assert_eq!(keyed["keys"], 2);
    assert_eq!(keyed["bones"], json!(["chest", "head"]));
    let over = keyed["overLimits"].as_array().expect("over");
    assert_eq!(over.len(), 1, "{keyed}");
    assert_eq!(over[0]["bone"], "chest");
    assert_eq!(over[0]["frame"], 20);
    assert_eq!(over[0]["over"][0]["word"], "twist");
    assert_eq!(over[0]["over"][0]["limit"], 45.0);

    // Halfway between the keys, out eases three quarters of the way.
    let frames = keyed["frames"].as_u64().expect("frames");
    assert!(frames > 20, "{keyed}");
    let pose = call(
        &client,
        "get_recipe_pose",
        json!({ "project": PROJECT, "recipe": id, "frame": 12 }),
    )
    .await;
    let chest = pose["bones"]
        .as_array()
        .expect("bones")
        .iter()
        .find(|b| b["bone"] == "chest")
        .expect("chest")
        .clone();
    assert_eq!(chest["layer"], layer.as_str());
    assert_eq!(chest["turn"], json!([0.0, 41.5, 0.0]), "{pose}");
    assert_eq!(chest["limits"]["y"], json!([-45.0, 45.0]));
    assert_eq!(chest["over"], json!([]));
    assert_eq!(pose["at"], 12);

    // Move the head on its own, then take the whole first key out; mode and weight change.
    let moved = call(
        &client,
        "set_recipe_keys",
        json!({
            "project": PROJECT, "recipe": id, "layer": layer, "expected": keyed["version"],
            "mode": "replace", "weight": 0.5,
            "move": [{ "frame": 4, "toFrame": 8, "bones": ["head"] }],
        }),
    )
    .await;
    assert_eq!(moved["keys"], 3, "{moved}");
    assert_eq!(moved["layer"]["mode"], "replace");
    assert_eq!(moved["layer"]["weight"], 0.5);
    let removed = call(
        &client,
        "set_recipe_keys",
        json!({
            "project": PROJECT, "recipe": id, "layer": layer, "expected": moved["version"],
            "remove": [{ "frame": 4 }, { "frame": 8, "bones": ["head"] }],
        }),
    )
    .await;
    assert_eq!(removed["keys"], 1, "{removed}");
    assert_eq!(removed["bones"], json!(["chest"]));

    // A stale version, a key that is not there and nothing to do are refused.
    let stale = refused(
        &client,
        "set_recipe_keys",
        json!({ "project": PROJECT, "recipe": id, "layer": layer, "expected": keyed["version"],
                "set": [{ "frame": 2, "pose": { "spine": [5.0, 0.0, 0.0] } }] }),
    )
    .await;
    assert_eq!(stale["code"], "moved", "{stale}");
    let missing = refused(
        &client,
        "set_recipe_keys",
        json!({ "project": PROJECT, "recipe": id, "layer": layer, "expected": removed["version"],
                "remove": [{ "frame": 30 }] }),
    )
    .await;
    assert!(
        missing["error"]
            .as_str()
            .is_some_and(|e| e.contains("no key on frame 30")),
        "{missing}"
    );
    let empty = refused(
        &client,
        "set_recipe_keys",
        json!({ "project": PROJECT, "recipe": id, "expected": removed["version"] }),
    )
    .await;
    assert!(
        empty["error"]
            .as_str()
            .is_some_and(|e| e.contains("Send set")),
        "{empty}"
    );

    // Pose keys bake now.
    let read = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    assert!(read.get("bakeable").is_none(), "pose keys bake: {read}");
    server.stop();
}
