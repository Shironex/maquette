//! get_started's short answer: counts and what stands in the way, with the
//! detail an agent asks for with `detail: true`.

use serde_json::{Map, Value, json};

/// What `detail: true` adds, said in the short answer.
const MORE: &str = "Pass detail true for every tool with its version, each project's template, repo, stages and activity, the image backend's settings and every approval a person gives (paid sends and image rounds, Ship, new dev servers, avatars). get_image_backend and list_projects read those parts alone.";

fn works(tool: &Value) -> bool {
    tool["status"] == "found" && tool["warning"].is_null()
}

/// Keeps only `keys` of an object.
fn pick(value: &Value, keys: &[&str]) -> Value {
    let mut out = Map::new();
    for key in keys {
        if let Some(v) = value.get(*key) {
            out.insert((*key).to_string(), v.clone());
        }
    }
    Value::Object(out)
}

/// The full answer cut down to counts, problems and next steps.
pub(super) fn compact(mut full: Value) -> Value {
    if let Some(tools) = full["toolchain"].as_array() {
        let problems: Vec<Value> = tools
            .iter()
            .filter(|t| !works(t))
            .map(|t| pick(t, &["id", "label", "required", "status", "hint", "warning"]))
            .collect();
        full["toolchain"] = json!({
            "working": tools.iter().filter(|t| works(t)).count(),
            "of": tools.len(),
            "problems": problems,
        });
    }
    if full["imageBackend"].is_object() {
        full["imageBackend"] = pick(&full["imageBackend"], &["backends", "keySaved"]);
    }
    if let Some(projects) = full["projects"].as_array() {
        let short: Vec<Value> = projects
            .iter()
            .map(|p| pick(p, &["id", "name", "target", "stage", "assets"]))
            .collect();
        full["projects"] = Value::Array(short);
    }
    full["more"] = json!(MORE);
    full
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_short_answer_keeps_counts_problems_and_next() {
        let full = json!({
            "workspace": "~/Maquette",
            "toolchain": [
                { "id": "blender", "label": "Blender", "required": true, "status": "found", "version": "4.5", "hint": "h", "warning": null },
                { "id": "codex", "label": "Codex", "required": false, "status": "missing", "version": null, "hint": "install it", "warning": null },
                { "id": "ktx", "label": "KTX", "required": false, "status": "found", "version": "4", "hint": "h", "warning": "old" },
            ],
            "imageBackend": { "backends": { "concepts": "codex" }, "keySaved": false, "model": "gpt", "estimateUsdPerImage": {} },
            "projects": [{ "id": "moon", "name": "Moon", "target": "website", "template": "room", "stage": "empty", "assets": 0, "stages": {}, "activity": {} }],
            "next": [{ "tool": "create_project" }],
        });
        let short = compact(full);
        assert_eq!(short["toolchain"]["working"], 1);
        assert_eq!(short["toolchain"]["of"], 3);
        let problems = short["toolchain"]["problems"].as_array().expect("problems");
        assert_eq!(problems.len(), 2);
        assert_eq!(problems[0]["hint"], "install it");
        assert!(problems[0].get("version").is_none());
        assert_eq!(
            short["imageBackend"],
            json!({ "backends": { "concepts": "codex" }, "keySaved": false })
        );
        assert_eq!(
            short["projects"][0],
            json!({ "id": "moon", "name": "Moon", "target": "website", "stage": "empty", "assets": 0 })
        );
        assert_eq!(short["next"][0]["tool"], "create_project");
        assert!(
            short["more"]
                .as_str()
                .is_some_and(|m| m.contains("detail true"))
        );
    }
}
