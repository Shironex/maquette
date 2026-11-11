//! A real MCP client and recipe bakes: a recipe with open blanks is refused
//! with a code and a fix naming them, a recipe never baked reads as no bake,
//! and the test host, which runs no bake service, says so rather than
//! starting one. Adding a bake refuses one out of date and otherwise waits
//! for a person. Nothing runs Blender.

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
            "baseClip": "standing-dodge-left", "baseHash": "abc", "recipeBaseHash": null,
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
    // Its recipe changed since (version v1 is no hash of the file).
    assert_eq!(
        done["bake"]["stale"],
        maquette_core::animations::recipes::bake::RECIPE_CHANGED,
        "{done}"
    );
    assert_eq!(done["next"][0]["tool"], "bake_recipe", "{done}");

    client.cancel().await.expect("close");
    server.stop();
}

/// Writes `record` as the saved bake of `recipe`, with a staged model.
fn save_bake(dir: &std::path::Path, recipe: &str, record: &Value) {
    let root = dir.join("ws");
    let config = maquette_core::workspace::open(&root).expect("config");
    let folder = root
        .join(&config.layout.cache)
        .join(maquette_core::workspace::RECIPE_BAKES_DIR)
        .join(PROJECT)
        .join(recipe);
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::write(folder.join("bake.json"), record.to_string()).expect("record");
    std::fs::write(
        folder.join(maquette_core::animations::recipes::bake::STAGED_FILE),
        "staged",
    )
    .expect("staged");
}

#[tokio::test(flavor = "multi_thread")]
async fn adding_a_bake_refuses_one_out_of_date_and_otherwise_waits_for_a_person() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, _host) = serve(fixture(dir.path())).await;
    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left" }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();
    let args = json!({ "project": PROJECT, "recipe": id });

    let none = raw_call(&client, "add_recipe_bake", args.clone()).await;
    assert_eq!(none.is_error, Some(true));
    assert!(text(&none).contains("no bake"), "{}", text(&none));

    // Baked from another version of the recipe: out of date.
    save_bake(dir.path(), &id, &done_record(&id));
    let stale = raw_call(&client, "add_recipe_bake", args.clone()).await;
    assert_eq!(stale.is_error, Some(true));
    let body: Value = serde_json::from_str(&text(&stale)).expect("json");
    assert_eq!(body["code"], "stale-bake", "{body}");
    assert_eq!(body["tryTool"], "bake_recipe", "{body}");
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|e| e.starts_with(maquette_core::animations::recipes::bake::OUT_OF_DATE)),
        "{body}"
    );

    // Baked from this version onto the newest model: it waits for a person.
    let mut fresh = done_record(&id);
    fresh["result"]["recipeVersion"] = made["version"].clone();
    save_bake(dir.path(), &id, &fresh);
    let read = call(&client, "get_recipe_bake", args.clone()).await;
    assert_eq!(read["bake"]["stale"], Value::Null, "{read}");
    assert_eq!(read["bake"]["review"]["warns"], 0, "{read}");
    let feet = read["bake"]["review"]["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .find(|l| l["id"] == "feet")
        .cloned()
        .expect("feet");
    assert_eq!(feet["status"], "pass", "{feet}");
    assert_eq!(feet["max"], 2.0, "{feet}");
    assert_eq!(read["next"][0]["tool"], "add_recipe_bake", "{read}");

    let waiting = call(&client, "add_recipe_bake", args.clone()).await;
    assert_eq!(waiting["status"], "waiting for approval", "{waiting}");
    assert_eq!(waiting["pollEverySeconds"], 5, "{waiting}");
    assert!(
        waiting["whereToApprove"]
            .as_str()
            .is_some_and(|w| w.contains("Motion view")),
        "{waiting}"
    );
    assert!(!waiting.to_string().contains("staged.glb"), "{waiting}");
    let approval = waiting["approval"].as_str().expect("approval").to_string();
    let again = call(&client, "add_recipe_bake", args.clone()).await;
    assert_eq!(
        again["approval"],
        approval.as_str(),
        "one approval per bake"
    );
    let polled = call(&client, "get_recipe_bake", args).await;
    assert_eq!(polled["status"], "waiting for approval", "{polled}");
    assert_eq!(polled["approval"], approval.as_str(), "{polled}");
    assert_eq!(polled["bake"]["state"], "done", "nothing added by a tool");

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bake_that_pops_where_clips_meet_reads_with_its_seam_line_sheet_and_fix() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, _host) = serve(fixture(dir.path())).await;
    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left" }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();
    let mut record = done_record(&id);
    record["result"]["recipeVersion"] = made["version"].clone();
    record["result"]["seams"] = json!([{
        "kind": "join", "clip": 1, "at": null, "freeze": false, "first": 20, "last": 20,
        "frame": 20, "hipsCm": 5.1, "hipsAroundCm": 2.0, "hipsOverCm": 3.1, "bone": "chest",
        "turnDegrees": 6.0, "turnAroundDegrees": 4.0, "turnOverDegrees": 2.0, "pops": true,
        "about": "Dodge into Dodge", "blend": 0, "blendMost": 10, "ease": null, "easeMost": null,
    }]);
    record["result"]["sheet"] = json!([{
        "frame": 20, "kind": "seam", "label": "seam 1", "detail": "hips jump 5.1 cm", "warn": true,
    }]);
    save_bake(dir.path(), &id, &record);
    let read = call(
        &client,
        "get_recipe_bake",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    let review = &read["bake"]["review"];
    assert_eq!(review["warns"], 1, "{read}");
    let seam = &review["lines"][0];
    assert_eq!(seam["id"], "seam", "{review}");
    assert_eq!(seam["label"], "Seam pop");
    assert_eq!(seam["value"], "3.1 cm");
    assert_eq!(seam["status"], "warn");
    let fix = &review["fixes"][0];
    assert_eq!(fix["line"], "seam", "{review}");
    assert_eq!(fix["edit"]["kind"], "blend");
    assert_eq!(fix["edit"]["clip"], 1);
    assert!(
        fix["title"]
            .as_str()
            .is_some_and(|t| t.starts_with("Widen the blend to"))
    );
    assert_eq!(
        read["bake"]["result"]["sheet"][0]["label"], "seam 1",
        "{read}"
    );

    client.cancel().await.expect("close");
    server.stop();
}
