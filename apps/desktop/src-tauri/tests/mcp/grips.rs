//! A real MCP client and the grip library: it lists and reads grips, lays
//! one on a recipe with a link back, sees when the library has a newer
//! revision and takes it, and proposes saving the hand pose of a recipe's
//! pose keys, which only waits for a person: the library is written by the
//! person's save alone (the core save, as the app's sheet calls it), and no
//! tool removes a grip. Every refusal is checked. Everything happens in a
//! temp workspace; no Blender runs.

use std::sync::Arc;

use maquette_core::animations::grips::{self, GripFromKeys, GripSave, GripSaved};
use maquette_core::recipes::grips::GripFile;
use maquette_desktop_lib::mcp::hub::{ApprovalKind, Hub};
use serde_json::{Value, json};

use super::animations::fixture;
use super::start::serve_with;
use super::support::{Client, TestHost, call, call_error, raw_call, text};

const PROJECT: &str = "frostveil";

/// A refusal as the agent reads it, with its code and advice.
async fn refused(client: &Client, tool: &str, args: Value) -> Value {
    let result = raw_call(client, tool, args).await;
    assert_eq!(result.is_error, Some(true), "{tool} should fail");
    serde_json::from_str(&text(&result)).expect("error body")
}

/// A recipe on the library clip whose pose keys turn both hands, a finger
/// and the chest in place of the clip. Answers its id, the pose key
/// change's id and the recipe's version.
async fn keyed_recipe(client: &Client) -> (String, String, Value) {
    let made = call(
        client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left", "name": "Sword hold" }),
    )
    .await;
    let id = made["id"].as_str().expect("id").to_string();
    let keyed = call(
        client,
        "set_recipe_keys",
        json!({
            "project": PROJECT, "recipe": id, "expected": made["version"], "mode": "replace",
            "set": [
                { "frame": 10, "pose": {
                    "chest": [0.0, -18.0, 0.0],
                    "rightHand": [4.0, 0.0, -8.0],
                    "rightIndexProximal": [0.0, 0.0, 62.0],
                    "leftHand": [0.0, 6.0, 0.0],
                } },
                { "frame": 30, "pose": { "rightIndexProximal": [0.0, 0.0, 20.0] } },
            ],
        }),
    )
    .await;
    let layer = keyed["layer"]["id"].as_str().expect("layer").to_string();
    (id, layer, keyed["version"].clone())
}

/// The person's Save in the sheet: the grip the recipe's pose keys hold,
/// read and saved as the app does, which also ends the agent's ask.
fn person_saves(
    host: &TestHost,
    hub: &Hub,
    keys: GripFromKeys,
    entry: Option<&str>,
    change: impl FnOnce(&mut GripFile),
) -> GripSaved {
    let (root, config) = host.session.workspace().expect("ws");
    let mut keyed = grips::from_keys(&root, &config.layout, &keys).expect("keyed");
    change(&mut keyed.grip);
    let plan = grips::plan_save(&root, &keyed.grip, entry).expect("plan");
    let saved = grips::save_grip(
        &root,
        &GripSave {
            grip: keyed.grip,
            entry: plan.entry.clone(),
            expected: plan.entry_version,
            source: Some(keyed.source.clone()),
        },
    )
    .expect("save");
    hub.close_grip_save(PROJECT, &keys.recipe, &keyed.source.layer, entry);
    saved
}

fn keys_of(recipe: &str, layer: &str) -> GripFromKeys {
    GripFromKeys {
        project: PROJECT.into(),
        recipe: recipe.into(),
        layer: layer.into(),
        name: "Longsword, two hands".into(),
        weapon: "longsword".into(),
        ..GripFromKeys::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_propose_a_grip_lay_it_on_a_recipe_and_take_a_newer_revision() {
    let dir = tempfile::tempdir().expect("temp dir");
    let hub = Arc::new(Hub::new(|| {}));
    let (client, server, host) = serve_with(fixture(dir.path()), Arc::clone(&hub)).await;
    let (root, _) = host.session.workspace().expect("ws");

    let none = call(&client, "list_grips", json!({})).await;
    assert_eq!(none["grips"], json!([]));
    assert!(
        none["weapons"]
            .as_array()
            .is_some_and(|w| w.contains(&json!("longsword")) && w.contains(&json!("empty"))),
        "{none}"
    );
    assert_eq!(none["next"][0]["tool"], "propose_grip_save", "{none}");

    // A proposal waits for a person and writes nothing.
    let (recipe, keys, _) = keyed_recipe(&client).await;
    let ask = json!({
        "project": PROJECT, "recipe": recipe, "layer": keys,
        "name": "Longsword, two hands", "weapon": "longsword",
    });
    let waiting = call(&client, "propose_grip_save", ask.clone()).await;
    assert_eq!(waiting["status"], "waiting for approval", "{waiting}");
    assert!(
        waiting["whereToApprove"]
            .as_str()
            .is_some_and(|w| w.contains("save a grip sheet") && w.contains("No tool can approve")),
        "{waiting}"
    );
    let plan = &waiting["plan"];
    assert_eq!(plan["entry"], Value::Null, "a new grip: {waiting}");
    assert_eq!(plan["revision"], 1);
    assert_eq!(
        plan["file"],
        "library/grips/longsword-two-hands/r1.grip.json"
    );
    assert_eq!(plan["frame"], 10, "the first key on a hand");
    assert_eq!(
        (&plan["holds"], &plan["supports"]),
        (&json!("right"), &json!("left"))
    );
    assert_eq!(plan["bones"], 3, "the chest is not a grip's: {waiting}");
    assert_eq!(plan["pose"]["rightIndexProximal"], json!([0.0, 0.0, 62.0]));
    assert!(
        waiting.get("notes").is_none(),
        "replace keys need no note: {waiting}"
    );
    let approval = waiting["approval"].as_str().expect("approval").to_string();
    let again = call(&client, "propose_grip_save", ask.clone()).await;
    assert_eq!(
        again["approval"],
        approval.as_str(),
        "one approval per change"
    );
    assert!(
        !grips::dir(&root).exists(),
        "a proposal writes nothing to the library"
    );
    let cards = hub.approvals();
    assert_eq!(cards.len(), 1, "the app shows one card");
    assert_eq!(cards[0].kind, ApprovalKind::GripSave);
    assert_eq!(
        cards[0].asset_name.as_deref(),
        Some("Longsword, two hands r1")
    );
    let card = cards[0]
        .recipe_save
        .as_ref()
        .and_then(|a| a.grip.as_ref())
        .expect("ask");
    assert_eq!(
        (card.layer.as_str(), card.frame, card.latest),
        (keys.as_str(), 10, None)
    );
    assert_eq!(card.bones.len(), 3);
    let read = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": recipe }),
    )
    .await;
    assert_eq!(
        read["gripSavesWaiting"],
        json!([{ "layer": keys, "approval": approval }]),
        "{read}"
    );

    // The person saves in the sheet: the library has it, and nothing waits.
    let saved = person_saves(&host, &hub, keys_of(&recipe, &keys), None, |_| {});
    assert_eq!(
        (saved.entry.as_str(), saved.revision),
        ("longsword-two-hands", 1)
    );
    assert!(hub.approvals().is_empty(), "the save answers the card");
    let listed = call(&client, "list_grips", json!({})).await;
    let line = &listed["grips"][0];
    assert_eq!(line["id"], "longsword-two-hands", "{listed}");
    assert_eq!(
        (&line["weapon"], &line["group"]),
        (&json!("longsword"), &json!("swords"))
    );
    assert_eq!(
        (&line["hands"], &line["bones"], &line["latest"]),
        (&json!("both hands"), &json!(3), &json!(1))
    );
    assert_eq!(line["revisions"][0]["project"], PROJECT, "{listed}");
    assert_eq!(line["revisions"][0]["layer"], keys.as_str());
    assert_eq!(line["revisions"][0]["frame"], 10);
    assert!(line.get("pose").is_none(), "the list leaves poses out");
    assert_eq!(listed["next"][0]["tool"], "apply_grip");
    let grip = call(
        &client,
        "get_grip",
        json!({ "grip": "longsword-two-hands" }),
    )
    .await;
    assert_eq!((&grip["revision"], &grip["latest"]), (&json!(1), &json!(1)));
    assert_eq!(grip["pose"]["leftHand"], json!([0.0, 6.0, 0.0]), "{grip}");
    assert_eq!(grip["holds"], "right");
    let uid = grip["uid"].as_str().expect("uid").to_string();
    let same = call_error(
        &client,
        "propose_grip_save",
        json!({ "project": PROJECT, "recipe": recipe, "layer": keys,
                "entry": "longsword-two-hands", "weapon": "longsword" }),
    )
    .await;
    assert!(same.contains("nothing to save"), "{same}");

    // Laid on the recipe: a copy of the right hand and a link back.
    let version = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": recipe }),
    )
    .await["version"]
        .clone();
    let laid = call(
        &client,
        "apply_grip",
        json!({ "project": PROJECT, "recipe": recipe, "grip": "longsword-two-hands",
                "hands": "right", "expected": version, "start": 0.25, "end": 0.75 }),
    )
    .await;
    let change = &laid["layer"];
    let layer = change["id"].as_str().expect("layer id").to_string();
    assert_eq!(
        (&change["kind"], &change["bone"], &change["mode"]),
        (&json!("grip"), &json!("rightHand"), &json!("replace")),
        "{laid}"
    );
    assert_eq!(
        change["from"],
        json!({ "entry": "longsword-two-hands", "uid": uid, "revision": 1 })
    );
    assert_eq!(
        (&change["easeIn"], &change["easeOut"]),
        (&json!(4), &json!(4))
    );
    assert_eq!(
        (&laid["bones"], &laid["updated"], &laid["newer"]),
        (&json!(2), &json!(false), &json!(false))
    );
    assert!(
        change["pose"].get("leftHand").is_none(),
        "the right hand only"
    );
    let read = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": recipe }),
    )
    .await;
    assert_eq!(read["format"], 2, "{read}");
    assert!(read.get("bakeable").is_none(), "grips bake: {read}");
    assert_eq!(
        read["changes"][1]["library"],
        json!({ "name": "Longsword, two hands", "latest": 1, "newer": false }),
        "{read}"
    );
    assert!(
        read.get("gripSavesWaiting").is_none(),
        "saved, so nothing waits: {read}"
    );

    // r2 turns only the left hand another way: the recipe is offered it,
    // and taking it moves the link, not what the recipe's right hand does.
    person_saves(
        &host,
        &hub,
        keys_of(&recipe, &keys),
        Some("longsword-two-hands"),
        |g| {
            g.pose.insert("leftHand".into(), [0.0, 30.0, 0.0]);
        },
    );
    let offered = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": recipe }),
    )
    .await;
    assert_eq!(offered["changes"][1]["library"]["newer"], true, "{offered}");
    assert_eq!(offered["changes"][1]["library"]["latest"], 2);
    assert!(
        offered["next"]
            .as_array()
            .is_some_and(|n| n.iter().any(|s| s["tool"] == "apply_grip"
                && s["why"].as_str().is_some_and(|w| w.contains("r2")))),
        "{offered}"
    );
    let taken = call(
        &client,
        "apply_grip",
        json!({ "project": PROJECT, "recipe": recipe, "grip": "longsword-two-hands",
                "layer": layer, "expected": offered["version"] }),
    )
    .await;
    assert_eq!(
        (&taken["updated"], &taken["revision"], &taken["bones"]),
        (&json!(true), &json!(2), &json!(2)),
        "{taken}"
    );
    assert_eq!(
        taken["motionVersion"], laid["motionVersion"],
        "a link and its revision stay out of the motion version"
    );
    assert_eq!(taken["layer"]["start"], json!(0.25), "where it sat");

    // r3 curls the right index: taking it changes the motion.
    person_saves(
        &host,
        &hub,
        keys_of(&recipe, &keys),
        Some("longsword-two-hands"),
        |g| {
            g.pose.insert("leftHand".into(), [0.0, 30.0, 0.0]);
            g.pose.insert("rightIndexProximal".into(), [0.0, 0.0, 80.0]);
        },
    );
    let curled = call(
        &client,
        "apply_grip",
        json!({ "project": PROJECT, "recipe": recipe, "grip": "longsword-two-hands",
                "layer": layer, "expected": taken["version"] }),
    )
    .await;
    assert_eq!(curled["revision"], 3, "{curled}");
    assert_ne!(
        curled["motionVersion"], taken["motionVersion"],
        "the pose counts"
    );

    // The recipe with its grip is taken by a bake: only this test's host,
    // which runs no bakes, refuses.
    let bake = refused(
        &client,
        "bake_recipe",
        json!({ "project": PROJECT, "recipe": recipe, "asset": "dummy" }),
    )
    .await;
    assert_eq!(
        bake["error"], "Recipe bakes are not available in this session.",
        "{bake}"
    );

    // The person removes the grip: the recipe keeps its copy, the link is gone.
    grips::remove_entry(&root, "longsword-two-hands", None).expect("remove");
    let gone = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": recipe }),
    )
    .await;
    assert_eq!(
        gone["changes"][1]["library"],
        json!({ "gone": true }),
        "{gone}"
    );
    assert_eq!(gone["changes"][1]["pose"]["rightIndexProximal"][2], 80.0);
    assert_eq!(
        call(&client, "list_grips", json!({})).await["grips"],
        json!([])
    );

    client.cancel().await.expect("close");
    server.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_grip_tools_refuse_what_does_not_check_out_and_none_removes_or_saves() {
    let dir = tempfile::tempdir().expect("temp dir");
    let hub = Arc::new(Hub::new(|| {}));
    let (client, server, host) = serve_with(fixture(dir.path()), Arc::clone(&hub)).await;
    let (recipe, keys, version) = keyed_recipe(&client).await;
    person_saves(&host, &hub, keys_of(&recipe, &keys), None, |_| {});
    let grip = "longsword-two-hands";

    // No tool saves to the grip library or removes from it.
    let tools: Vec<String> = client
        .list_all_tools()
        .await
        .expect("tools")
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    let mut about_grips: Vec<&String> = tools.iter().filter(|n| n.contains("grip")).collect();
    about_grips.sort();
    assert_eq!(
        about_grips,
        ["apply_grip", "get_grip", "list_grips", "propose_grip_save"],
        "the four the design names"
    );

    // Reading a grip or a revision that is not there.
    for args in [
        json!({ "grip": "no-such-grip" }),
        json!({ "grip": grip, "revision": 9 }),
        json!({ "grip": "../recipes" }),
    ] {
        let missing = refused(&client, "get_grip", args.clone()).await;
        assert_eq!(missing["code"], "not-found", "{args}: {missing}");
    }

    // Laying a grip: each refusal leaves the recipe as it was.
    let apply = |extra: Value| {
        let mut args = json!({ "project": PROJECT, "recipe": recipe, "grip": grip,
                               "expected": version });
        for (key, value) in extra.as_object().expect("object") {
            args[key] = value.clone();
        }
        args
    };
    for (extra, code, words) in [
        (
            json!({ "grip": "no-such-grip" }),
            "not-found",
            "library grip",
        ),
        (json!({ "revision": 9 }), "not-found", "revision r9"),
        (
            json!({ "expected": "0000000000000000" }),
            "moved",
            "changed since",
        ),
        (json!({ "layer": keys }), "invalid", "not a grip change"),
        (
            json!({ "hands": "three" }),
            "refused",
            "both, right or left",
        ),
        (json!({ "mode": "over" }), "refused", "replace or add"),
        (json!({ "target": "hilt" }), "invalid", "the blank hilt"),
        (
            json!({ "start": 0.8, "end": 0.2 }),
            "invalid",
            "start before it ends",
        ),
        (
            json!({ "weight": 2.0 }),
            "invalid",
            "weight has to be 0 to 1",
        ),
    ] {
        let body = refused(&client, "apply_grip", apply(extra.clone())).await;
        assert_eq!(body["code"], code, "{extra}: {body}");
        assert!(
            body["error"].as_str().is_some_and(|e| e.contains(words)),
            "{extra}: {body}"
        );
    }
    let moved = refused(
        &client,
        "apply_grip",
        apply(json!({ "expected": "0000000000000000" })),
    )
    .await;
    assert_eq!(moved["tryTool"], "get_recipe", "{moved}");
    let unchanged = call(
        &client,
        "get_recipe",
        json!({ "project": PROJECT, "recipe": recipe }),
    )
    .await;
    assert_eq!(unchanged["version"], version, "nothing was written");

    // Proposing a save: each refusal raises no card.
    let propose = |extra: Value| {
        let mut args = json!({ "project": PROJECT, "recipe": recipe, "layer": keys,
                               "name": "Another grip", "weapon": "longsword" });
        for (key, value) in extra.as_object().expect("object") {
            args[key] = value.clone();
        }
        args
    };
    for (extra, code, words) in [
        (
            json!({ "name": "  " }),
            "refused",
            "Give the new grip a name",
        ),
        (
            json!({ "weapon": "lightsaber" }),
            "invalid",
            "not a weapon a grip holds",
        ),
        (json!({ "layer": "l-9" }), "not-found", "the change l-9"),
        (
            json!({ "bones": ["chest"] }),
            "invalid",
            "turn no hand, finger or arm bone",
        ),
        (
            json!({ "hands": "sideways" }),
            "refused",
            "both, right or left",
        ),
        (json!({ "holds": "both" }), "refused", "right or left"),
        (
            json!({ "entry": "no-such-grip" }),
            "not-found",
            "library grip",
        ),
        (json!({ "entry": grip }), "refused", "nothing to save"),
        (json!({ "recipe": "no-such-recipe" }), "not-found", "recipe"),
    ] {
        let body = refused(&client, "propose_grip_save", propose(extra.clone())).await;
        assert_eq!(body["code"], code, "{extra}: {body}");
        assert!(
            body["error"].as_str().is_some_and(|e| e.contains(words)),
            "{extra}: {body}"
        );
    }
    // One hand keyed, both asked for.
    let one_hand = call(
        &client,
        "set_recipe_keys",
        json!({ "project": PROJECT, "recipe": recipe, "expected": version, "mode": "add",
                "set": [{ "frame": 5, "pose": { "rightHand": [0.0, 10.0, 0.0] } }] }),
    )
    .await;
    let other = one_hand["layer"]["id"].as_str().expect("layer").to_string();
    let both = refused(
        &client,
        "propose_grip_save",
        propose(json!({ "layer": other, "hands": "both" })),
    )
    .await;
    assert!(
        both["error"]
            .as_str()
            .is_some_and(|e| e.contains("no bone on the left hand")),
        "{both}"
    );
    assert!(hub.approvals().is_empty(), "no refusal raised a card");
    // Keys that add to the clip are read as they are, and the answer says so.
    let noted = call(
        &client,
        "propose_grip_save",
        propose(json!({ "layer": other })),
    )
    .await;
    assert_eq!(noted["plan"]["supports"], Value::Null, "{noted}");
    assert!(
        noted["notes"][0]
            .as_str()
            .is_some_and(|n| n.contains("add to the clip")),
        "{noted}"
    );
    assert_eq!(hub.approvals().len(), 1);

    client.cancel().await.expect("close");
    server.stop();
}
