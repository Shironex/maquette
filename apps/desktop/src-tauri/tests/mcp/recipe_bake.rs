//! A real MCP client and recipe bakes: a recipe with open blanks is refused
//! with a code and a fix naming them, a recipe never baked reads as no bake,
//! and the test host, which runs no bake service, says so rather than
//! starting one. Nothing runs Blender.

use serde_json::{Value, json};

use super::animations::fixture;
use super::start::serve;
use super::support::{call, call_error, raw_call, text};

const PROJECT: &str = "frostveil";

/// A finished bake's record as the service saves it.
fn done_record(recipe: &str) -> Value {
    json!({
        "project": PROJECT, "recipe": recipe, "recipeName": "Dodge", "asset": "dummy",
        "state": "done", "startedBy": "person", "progress": null, "error": null,
        "startedMs": 1.0, "updatedMs": 2.0, "finishedMs": 2.0,
        "result": {
            "recipe": recipe, "recipeName": "Dodge", "recipeVersion": "v1",
            "baseClip": "standing-dodge-left", "baseHash": "h1", "recipeBaseHash": null,
            "mixamo": true, "asset": "dummy", "assetName": "Dummy", "fromVersion": 1,
            "clip": "reach_out", "frames": 31, "file": "/ws/cache/recipes/frostveil/x/staged.glb",
            "facing": [0.0, 0.0, 1.0], "slideCm": 0.5, "slideFrames": null, "slideFoot": null,
            "baseSlideCm": 0.5, "stretchMm": null, "stretchFrame": null, "baseStretchMm": null,
            "stretchNote": null, "reaches": [], "blenderSeconds": 1.0, "seconds": 1.5,
            "warnings": [],
        },
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bake_names_the_open_blanks_and_a_recipe_never_baked_reads_as_none() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, _host) = serve(fixture(dir.path())).await;
    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left" }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();
    let open = call(
        &client,
        "update_recipe",
        json!({
            "project": PROJECT, "recipe": id, "expected": made["version"],
            "blanks": [
                { "id": "cup", "label": "The cup", "kind": "prop" },
                { "id": "spot", "label": "Where to look", "kind": "point" },
            ],
            "changes": [
                { "kind": "reach", "bone": "rightHand", "target": "cup" },
                { "kind": "look-at", "bone": "head", "target": "spot" },
            ],
        }),
    )
    .await;

    let refused = raw_call(
        &client,
        "bake_recipe",
        json!({ "project": PROJECT, "recipe": id, "asset": "dummy" }),
    )
    .await;
    assert_eq!(refused.is_error, Some(true));
    let body: Value = serde_json::from_str(&text(&refused)).expect("json");
    assert_eq!(body["code"], "open-blanks", "{body}");
    assert_eq!(body["tryTool"], "update_recipe", "{body}");
    let says = body["error"].as_str().expect("error");
    assert!(says.contains("cup, spot"), "{says}");
    assert!(
        body["fix"]
            .as_str()
            .is_some_and(|f| f.contains("update_recipe"))
    );

    let none = call(
        &client,
        "get_recipe_bake",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    assert_eq!(none["bake"], Value::Null, "{none}");

    call(
        &client,
        "update_recipe",
        json!({
            "project": PROJECT, "recipe": id, "expected": open["version"],
            "blanks": [
                { "id": "cup", "label": "The cup", "kind": "prop", "prop": "dummy" },
                { "id": "spot", "label": "Where to look", "kind": "point", "point": [0.0, 1.5, 1.0] },
            ],
        }),
    )
    .await;
    let unserved = call_error(
        &client,
        "bake_recipe",
        json!({ "project": PROJECT, "recipe": id, "asset": "dummy" }),
    )
    .await;
    assert!(unserved.contains("not available"), "{unserved}");

    // A finished bake reads without the staged file's path: only a person's
    // review adds it.
    let root = dir.path().join("ws");
    let config = maquette_core::workspace::open(&root).expect("config");
    let folder = root
        .join(&config.layout.cache)
        .join(maquette_core::workspace::RECIPE_BAKES_DIR)
        .join(PROJECT)
        .join(&id);
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::write(folder.join("bake.json"), done_record(&id).to_string()).expect("record");
    let done = call(
        &client,
        "get_recipe_bake",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    assert_eq!(done["bake"]["state"], "done", "{done}");
    assert_eq!(done["bake"]["result"]["clip"], "reach_out", "{done}");
    assert!(done["bake"]["result"].get("file").is_none(), "{done}");
    assert!(!done.to_string().contains("staged.glb"), "{done}");

    client.cancel().await.expect("close");
    server.stop();
}
