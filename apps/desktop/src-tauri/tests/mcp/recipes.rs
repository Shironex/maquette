//! A real MCP client makes an animation recipe on a library clip, fills its
//! blanks and changes, is refused a change that does not check out with the
//! file left as it was, renames it, sees a base clip whose keys changed as
//! a warning, sees a prop blank open again once its asset is gone and can
//! still change the recipe, and neither sees nor runs a recipe tool while
//! the animation group is off. Everything happens in a temp workspace; no
//! Blender runs.

use maquette_core::project;
use serde_json::{Value, json};

use super::animations::fixture;
use super::start::serve;
use super::support::{call, call_error, raw_call, text};

const PROJECT: &str = "frostveil";

fn steps(answer: &Value) -> Vec<String> {
    answer["next"]
        .as_array()
        .map(|n| {
            n.iter()
                .filter_map(|s| s["tool"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// An update_recipe refusal as the agent reads it: error, code, fix, tryTool.
async fn error_body(client: &super::support::Client, args: Value) -> Value {
    let result = raw_call(client, "update_recipe", args).await;
    assert_eq!(result.is_error, Some(true), "update_recipe should fail");
    serde_json::from_str(&text(&result)).expect("json")
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_make_a_recipe_fill_it_and_are_refused_what_does_not_check_out() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, host) = serve(fixture(dir.path())).await;

    let empty = call(&client, "list_recipes", json!({ "project": PROJECT })).await;
    assert_eq!(empty["recipes"], json!([]));
    assert!(
        steps(&empty).contains(&"create_recipe".to_string()),
        "{empty}"
    );

    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left" }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();
    assert_eq!(id, "standing-dodge-left");
    assert_eq!(made["file"], "recipes/standing-dodge-left.recipe.json");
    assert_eq!(made["mixamo"], true);
    assert_eq!(made["base"]["frames"], 38);
    assert_eq!(made["base"]["library"], "found");
    assert_eq!(made["changes"], json!([]));
    assert!(steps(&made).contains(&"update_recipe".to_string()));

    let unversioned = error_body(
        &client,
        json!({ "project": PROJECT, "recipe": id,
                "changes": [{ "kind": "nudge", "bone": "spine", "rotation": [0.0, 10.0, 0.0] }] }),
    )
    .await;
    assert_eq!(
        (
            unversioned["code"].as_str(),
            unversioned["tryTool"].as_str()
        ),
        (Some("needs-version"), Some("get_recipe")),
        "{unversioned}"
    );
    let filled = call(
        &client,
        "update_recipe",
        json!({
            "project": PROJECT, "recipe": id, "expected": made["version"],
            "blanks": [{ "id": "target", "label": "What the hand reaches", "kind": "prop", "prop": "dummy" }],
            "changes": [
                { "kind": "reach", "bone": "rightHand", "target": "target", "start": 0.1, "end": 0.5 },
                { "kind": "look at", "bone": "head", "target": "target" },
                { "kind": "nudge", "bone": "spine", "rotation": [0.0, 10.0, 0.0] },
            ],
        }),
    )
    .await;
    let changes = filled["changes"].as_array().expect("changes");
    let ids: Vec<&str> = changes.iter().filter_map(|c| c["id"].as_str()).collect();
    assert_eq!(ids, ["l-1", "l-2", "l-3"]);
    assert_eq!(changes[1]["kind"], "look-at");
    assert_eq!(changes[0]["start"], 0.1);
    assert!(changes[2].get("target").is_none(), "{filled}");
    assert_eq!(filled["blanks"][0]["filled"], true);

    for (change, says) in [
        (
            json!({ "kind": "spin", "bone": "head" }),
            "not a kind of change",
        ),
        (
            json!({ "kind": "reach", "bone": "head", "target": "target" }),
            "A reach moves",
        ),
        (
            json!({ "kind": "wave", "bone": "rightHand" }),
            "needs cycles",
        ),
    ] {
        let refused = call_error(
            &client,
            "update_recipe",
            json!({ "project": PROJECT, "recipe": id, "changes": [change],
                    "expected": filled["version"] }),
        )
        .await;
        assert!(refused.contains(says), "{refused}");
    }
    let stranger = call_error(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "name": "Grab it", "expected": filled["version"],
                "blanks": [{ "id": "target", "label": "Prop", "kind": "prop", "prop": "nope" }] }),
    )
    .await;
    assert!(stranger.contains("not an asset"), "{stranger}");
    let kept = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    assert_eq!(kept["changes"].as_array().map(Vec::len), Some(3), "{kept}");
    assert_eq!(
        kept["name"], "Standing Dodge Left",
        "a refused call renames nothing"
    );

    let renamed = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "name": "Grab it" }),
    )
    .await;
    assert_eq!(
        (renamed["id"].as_str(), renamed["name"].as_str()),
        (Some(id.as_str()), Some("Grab it"))
    );
    assert_eq!(renamed["revision"], 1);
    let moved = error_body(
        &client,
        json!({ "project": PROJECT, "recipe": id, "name": "Mine",
                "expected": kept["version"] }),
    )
    .await;
    assert_eq!(
        (moved["code"].as_str(), moved["tryTool"].as_str()),
        (Some("moved"), Some("get_recipe")),
        "{moved}"
    );
    let fresh = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "name": "Grab it now",
                "expected": renamed["version"] }),
    )
    .await;
    assert_eq!(fresh["name"], "Grab it now");

    let (root, _) = host.session.workspace().expect("ws");
    let meta = root.join("library/animations/standing-dodge-left/clip.json");
    let mut clip: Value =
        serde_json::from_str(&std::fs::read_to_string(&meta).expect("read")).expect("json");
    clip["hash"] = json!("new-keys");
    std::fs::write(&meta, clip.to_string()).expect("write");
    let changed = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    assert_eq!(changed["base"]["library"], "hash-differs");
    assert!(changed["base"]["warning"].as_str().is_some(), "{changed}");

    let listed = call(&client, "list_recipes", json!({ "project": PROJECT })).await;
    let row = &listed["recipes"][0];
    assert_eq!(
        (row["changes"].as_u64(), row["open"].as_u64()),
        (Some(3), Some(0))
    );
    assert_eq!(row["version"], changed["version"], "{listed}");
    assert!(listed.get("next").is_none());

    let layout = host.session.workspace().expect("ws").1.layout;
    project::modify(&root, &layout, PROJECT, |m| {
        m.assets.clear();
        Ok(())
    })
    .expect("remove the dummy");
    let stale = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": id }),
    )
    .await;
    assert_eq!(stale["blanks"][0]["filled"], false, "{stale}");
    let warning = stale["warnings"][0].as_str().expect("warning");
    assert!(warning.contains("dummy"), "{warning}");
    assert!(steps(&stale).contains(&"update_recipe".to_string()));
    let listed = call(&client, "list_recipes", json!({ "project": PROJECT })).await;
    assert_eq!(listed["recipes"][0]["open"], 1, "{listed}");
    let still = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "expected": stale["version"],
                "changes": [{ "kind": "nudge", "bone": "spine", "rotation": [0.0, 5.0, 0.0] }] }),
    )
    .await;
    assert_eq!(
        still["changes"].as_array().map(Vec::len),
        Some(1),
        "{still}"
    );
    assert!(
        still["warnings"][0].as_str().is_some(),
        "it says which blank opened"
    );

    let gone = call_error(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "no-such-clip" }),
    )
    .await;
    assert!(gone.contains("no-such-clip"), "{gone}");

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_recipe_tools_are_hidden_and_refused_while_the_animation_group_is_off() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, host) = serve(fixture(dir.path())).await;
    host.session
        .update_config(|c| c.agents.rules.tool_groups_off = vec!["animation".into()])
        .expect("save");
    let listed: Vec<String> = client
        .list_all_tools()
        .await
        .expect("tools")
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(listed.iter().any(|n| n == "list_projects"));
    for tool in [
        "list_recipes",
        "get_recipe",
        "create_recipe",
        "update_recipe",
        "bake_recipe",
        "get_recipe_bake",
        "add_recipe_bake",
        "list_library_recipes",
        "get_library_recipe",
        "use_recipe",
        "propose_recipe_save",
        "import_recipe",
        "export_recipe",
    ] {
        let refused = raw_call(
            &client,
            tool,
            json!({ "project": PROJECT, "recipe": "x", "clip": "standing-dodge-left" }),
        )
        .await;
        assert!(!listed.iter().any(|n| n == tool), "{tool} is listed");
        assert_eq!(refused.is_error, Some(true), "{tool}");
        let body: Value = serde_json::from_str(&text(&refused)).expect("json");
        assert_eq!(body["code"], "group-off", "{tool}");
    }
    let (root, _) = host.session.workspace().expect("ws");
    assert!(!root.join("projects/frostveil/recipes").exists());

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_join_clips_add_time_edits_and_keys_and_the_bake_waits_for_them() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, host) = serve(fixture(dir.path())).await;

    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "name": "Dodge twice", "clips": [
            { "clip": "standing-dodge-left", "trim": [2, 30] },
            { "clip": "standing-dodge-left", "blend": { "frames": 6, "lineUp": "hips" } },
        ] }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();
    assert_eq!(made["format"], 2, "{made}");
    let clips = made["clips"].as_array().expect("clips");
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0]["trim"], json!([2, 30]));
    assert!(clips[0].get("blend").is_none(), "{made}");
    assert_eq!(clips[1]["blend"]["frames"], 6);
    assert_eq!(clips[1]["blend"]["lineUp"], "hips");
    assert_eq!(clips[1]["library"], "found");
    assert!(made.get("bakeable").is_none(), "joined clips bake: {made}");
    assert_eq!(made["frames"], 60, "28 kept, then 38 less a blend of 6");

    let keyed = call(
        &client,
        "update_recipe",
        json!({
            "project": PROJECT, "recipe": id, "expected": made["version"],
            "time": [{ "kind": "freeze", "clip": 1, "at": 20, "frames": 12 }],
            "changes": [
                { "kind": "pose", "keys": [{ "clip": 0, "frame": 10, "ease": "out",
                                              "pose": { "chest": [0.0, -18.0, 0.0] } }] },
                { "kind": "pin", "bone": "leftFoot", "start": 0.1, "end": 0.5, "flat": "heel" },
            ],
        }),
    )
    .await;
    let changes = keyed["changes"].as_array().expect("changes");
    assert_eq!(changes[0]["mode"], "add", "pose keys add unless told");
    assert!(changes[0].get("bone").is_none(), "{keyed}");
    assert_eq!(changes[0]["keys"][0]["ease"], "out");
    assert_eq!(changes[1]["flat"], "heel");
    assert_eq!(keyed["time"][0]["kind"], "freeze");
    assert_eq!(keyed["bakeable"], false, "pose keys and pins wait: {keyed}");
    assert_eq!(keyed["frames"], 72, "the freeze adds its 12");

    // A change read back resends as it is.
    let resent = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "expected": keyed["version"],
                "changes": keyed["changes"] }),
    )
    .await;
    assert_eq!(resent["version"], keyed["version"], "nothing moved");

    let past = raw_call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "expected": keyed["version"],
                "clips": [{ "keep": 0 }] }),
    )
    .await;
    let past: Value = serde_json::from_str(&text(&past)).expect("json");
    assert_eq!(
        past["code"], "clip-in-use",
        "the freeze sits on clip 2: {past}"
    );

    // Swapping the clips moves the key and the freeze with their clips, and
    // each kept clip keeps its own trim.
    let swapped = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "expected": keyed["version"],
                "clips": [{ "keep": 1 }, { "keep": 0 }] }),
    )
    .await;
    assert_eq!(swapped["clips"][1]["trim"], json!([2, 30]), "{swapped}");
    assert_eq!(swapped["changes"][0]["keys"][0]["clip"], 1, "{swapped}");
    assert_eq!(swapped["time"][0]["clip"], 0, "{swapped}");
    let keyed = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "expected": swapped["version"],
                "clips": [{ "keep": 1 }, { "keep": 0 }] }),
    )
    .await;

    let bake = raw_call(
        &client,
        "bake_recipe",
        json!({ "project": PROJECT, "recipe": id, "asset": "dummy" }),
    )
    .await;
    assert_eq!(bake.is_error, Some(true));
    let body: Value = serde_json::from_str(&text(&bake)).expect("json");
    assert_eq!(body["code"], "not-bakeable-yet", "{body}");
    assert_eq!(body["tryTool"], "update_recipe", "{body}");

    let plain = call(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "expected": keyed["version"],
                "clips": [{ "keep": 1 }], "time": [], "changes": [] }),
    )
    .await;
    assert!(plain.get("format").is_none(), "back to format 1: {plain}");
    let (root, _) = host.session.workspace().expect("ws");
    let text =
        std::fs::read_to_string(root.join(format!("projects/frostveil/recipes/{id}.recipe.json")))
            .expect("file");
    assert!(text.contains("\"version\": 1,"), "{text}");

    client.cancel().await.expect("close");
    server.stop();
}
