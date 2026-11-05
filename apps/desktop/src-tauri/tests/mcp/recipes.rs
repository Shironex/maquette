//! A real MCP client makes an animation recipe on a library clip, fills its
//! blanks and changes, is refused a change that does not check out with the
//! file left as it was, renames it, sees a base clip whose keys changed as
//! a warning, and is refused every recipe tool while the animation group is
//! off. Everything happens in a temp workspace; no Blender runs.

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

    let filled = call(
        &client,
        "update_recipe",
        json!({
            "project": PROJECT, "recipe": id,
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
            json!({ "project": PROJECT, "recipe": id, "changes": [change] }),
        )
        .await;
        assert!(refused.contains(says), "{refused}");
    }
    let stranger = call_error(
        &client,
        "update_recipe",
        json!({ "project": PROJECT, "recipe": id, "name": "Grab it",
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
    assert!(listed.get("next").is_none());

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
async fn the_recipe_tools_are_refused_while_the_animation_group_is_off() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, host) = serve(fixture(dir.path())).await;
    host.session
        .update_config(|c| c.agents.rules.tool_groups_off = vec!["animation".into()])
        .expect("save");
    for tool in [
        "list_recipes",
        "get_recipe",
        "create_recipe",
        "update_recipe",
    ] {
        let refused = raw_call(
            &client,
            tool,
            json!({ "project": PROJECT, "recipe": "x", "clip": "standing-dodge-left" }),
        )
        .await;
        assert_eq!(refused.is_error, Some(true), "{tool}");
        let body: Value = serde_json::from_str(&text(&refused)).expect("json");
        assert_eq!(body["code"], "group-off", "{tool}");
    }
    let (root, _) = host.session.workspace().expect("ws");
    assert!(!root.join("projects/frostveil/recipes").exists());

    client.cancel().await.expect("close");
    server.stop();
}
