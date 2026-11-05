//! A real MCP client proposes saving a recipe to the recipe library and is
//! told it waits for a person, with nothing written; once the person saves
//! (the core save, as the app's sheet calls it), it reads the library link,
//! lists and reads the library recipe, uses it in the project, exports it
//! under exports/recipes, imports it back into the library and the project,
//! and is refused an import from outside the workspace. Everything happens
//! in a temp workspace; no Blender runs.

use maquette_core::animations::recipes::library::{self, RecipeSave};
use serde_json::{Value, json};

use super::animations::fixture;
use super::start::serve;
use super::support::{call, call_error};

const PROJECT: &str = "frostveil";

#[tokio::test(flavor = "multi_thread")]
async fn agents_propose_a_save_use_export_and_import_library_recipes() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (client, server, host) = serve(fixture(dir.path())).await;
    let (root, config) = host.session.workspace().expect("ws");

    let made = call(
        &client,
        "create_recipe",
        json!({ "project": PROJECT, "clip": "standing-dodge-left", "name": "Reach out" }),
    )
    .await;
    let filled = call(
        &client,
        "update_recipe",
        json!({
            "project": PROJECT, "recipe": "reach-out", "expected": made["version"],
            "blanks": [{ "id": "target", "label": "What the hand reaches", "kind": "point", "point": [0.2, 1.4, 0.3] }],
            "changes": [{ "kind": "reach", "bone": "rightHand", "target": "target" }],
        }),
    )
    .await;
    assert_eq!(filled["blanks"][0]["filled"], true, "{filled}");

    let none = call(&client, "list_library_recipes", json!({})).await;
    assert_eq!(none["recipes"], json!([]));

    let args = json!({ "project": PROJECT, "recipe": "reach-out" });
    let waiting = call(&client, "propose_recipe_save", args.clone()).await;
    assert_eq!(waiting["status"], "waiting for approval", "{waiting}");
    assert!(
        waiting["whereToApprove"]
            .as_str()
            .is_some_and(|w| w.contains("save to library sheet")),
        "{waiting}"
    );
    assert_eq!(
        waiting["plan"]["entry"],
        Value::Null,
        "a new library recipe"
    );
    assert_eq!(waiting["plan"]["revision"], 1);
    let approval = waiting["approval"].as_str().expect("approval").to_string();
    let again = call(&client, "propose_recipe_save", args.clone()).await;
    assert_eq!(
        again["approval"],
        approval.as_str(),
        "one approval per recipe"
    );
    assert!(
        !library::dir(&root).exists(),
        "a proposal writes nothing to the library"
    );
    let read = call(&client, "get_recipe", args.clone()).await;
    assert_eq!(read["saveWaiting"], approval.as_str(), "{read}");
    assert!(read.get("library").is_none(), "{read}");

    // The person saves in the sheet.
    let plan = library::plan_save(&root, &config.layout, PROJECT, "reach-out", None).expect("plan");
    library::save_back(
        &root,
        &config.layout,
        &RecipeSave {
            project: PROJECT.into(),
            recipe: "reach-out".into(),
            entry: None,
            expected: None,
            recipe_expected: Some(plan.recipe_version),
        },
    )
    .expect("save");
    let linked = call(&client, "get_recipe", args.clone()).await;
    assert_eq!(linked["library"]["entry"], "reach-out", "{linked}");
    assert_eq!(linked["library"]["revision"], 1, "{linked}");
    assert_eq!(linked["library"]["newer"], false, "{linked}");

    let listed = call(&client, "list_library_recipes", json!({})).await;
    assert_eq!(listed["recipes"][0]["id"], "reach-out", "{listed}");
    assert_eq!(listed["recipes"][0]["latest"], 1, "{listed}");
    let entry = call(
        &client,
        "get_library_recipe",
        json!({ "entry": "reach-out" }),
    )
    .await;
    assert_eq!(entry["revision"], 1, "{entry}");
    assert_eq!(entry["revisions"][0]["project"], PROJECT, "{entry}");
    assert_eq!(entry["recipe"]["base"]["library"], "found", "{entry}");
    let same = call_error(&client, "propose_recipe_save", args.clone()).await;
    assert!(same.contains("nothing to save"), "{same}");

    let used = call(
        &client,
        "use_recipe",
        json!({ "project": PROJECT, "entry": "reach-out" }),
    )
    .await;
    assert_eq!(used["id"], "reach-out-2", "{used}");
    assert!(
        used["notes"].as_array().is_some_and(|n| n
            .iter()
            .any(|l| l.as_str().is_some_and(|l| l.contains("reach-out-2")))),
        "{used}"
    );

    let exported = call(
        &client,
        "export_recipe",
        json!({ "project": PROJECT, "recipe": "reach-out", "file": "../../Reach out" }),
    )
    .await;
    assert_eq!(
        exported["path"], "exports/recipes/reach-out.recipe.json",
        "{exported}"
    );
    let file = root.join("exports/recipes/reach-out.recipe.json");
    let text = std::fs::read_to_string(&file).expect("export");
    assert!(!text.contains("\"from\""), "{text}");
    let second = call(&client, "export_recipe", json!({ "entry": "reach-out" })).await;
    assert_eq!(
        second["path"], "exports/recipes/reach-out-2.recipe.json",
        "never over a file"
    );

    let into_library = call(
        &client,
        "import_recipe",
        json!({ "path": "exports/recipes/reach-out.recipe.json" }),
    )
    .await;
    assert_eq!(into_library["entry"], "reach-out-2", "{into_library}");
    let into_project = call(
        &client,
        "import_recipe",
        json!({ "project": PROJECT, "path": file.display().to_string() }),
    )
    .await;
    assert_eq!(into_project["id"], "reach-out-3", "{into_project}");

    let outside = tempfile::tempdir().expect("outside");
    let elsewhere = outside.path().join("x.recipe.json");
    std::fs::copy(&file, &elsewhere).expect("copy");
    let refused = call_error(
        &client,
        "import_recipe",
        json!({ "path": elsewhere.display().to_string() }),
    )
    .await;
    assert!(refused.contains("inside the workspace"), "{refused}");

    client.cancel().await.expect("close");
    server.stop();
}
