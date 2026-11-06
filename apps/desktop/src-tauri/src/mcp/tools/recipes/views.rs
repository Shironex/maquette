//! What the recipe tools answer: a recipe, its clips and changes as an
//! agent reads them, and the next steps after it.

use maquette_core::animations::recipes::bake::not_bakeable;
use maquette_core::animations::recipes::{BaseMatch, BaseStatus, RecipeDoc};
use maquette_core::recipes::{LayerKind, RecipeBase, RecipeLayer};
use serde_json::{Map, Value, json};

use super::super::hints::step;
use super::super::{to_short_value, to_value};

/// One change in a few fields, as update_recipe takes it back; settings
/// and fields its kind does not use are left out.
pub(in crate::mcp::tools) fn change_view(layer: &RecipeLayer) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(layer.id));
    out.insert("kind".into(), to_value(layer.kind));
    if !layer.bone.is_empty() {
        out.insert("bone".into(), json!(layer.bone));
    }
    let settings = to_short_value(&layer.settings);
    if let Value::Object(fields) = settings {
        out.extend(fields.into_iter().filter(|(_, v)| !v.is_null()));
    }
    if let Some(mode) = layer.mode {
        out.insert("mode".into(), to_value(mode));
    }
    if layer.kind == LayerKind::Pose {
        out.insert("keys".into(), to_short_value(&layer.keys));
    }
    if let Some(pose) = &layer.pose {
        out.insert("pose".into(), to_short_value(pose));
    }
    if let Some(from) = &layer.from {
        out.insert("from".into(), to_value(from));
    }
    Value::Object(out)
}

/// One clip as agents read it, with how it stands in the library.
pub(super) fn clip_view(clip: &RecipeBase, found: &BaseMatch) -> Value {
    let mut out = json!({
        "name": clip.name, "frames": clip.frames,
        "seconds": to_short_value(clip.length), "hash": clip.hash,
        "library": to_value(found.status),
    });
    if clip.rest {
        out["rest"] = json!(true);
    }
    if let Some(id) = &found.clip {
        out["clip"] = json!(id);
    }
    if let Some(warning) = &found.warning {
        out["warning"] = json!(warning);
    }
    if let Some(trim) = clip.trim {
        out["trim"] = json!(trim);
    }
    if let Some(blend) = &clip.blend {
        out["blend"] = to_value(blend);
    }
    out
}

/// A recipe as agents read it.
pub(in crate::mcp::tools) fn recipe_view(doc: &RecipeDoc) -> Value {
    let recipe = &doc.recipe;
    let base = clip_view(&recipe.base, &doc.base);
    let blanks: Vec<Value> = recipe
        .blanks
        .iter()
        .map(|b| {
            json!({
                "id": b.id, "label": b.label, "kind": to_value(b.kind),
                "prop": b.prop, "point": b.point.map(to_short_value), "filled": b.filled(),
            })
        })
        .collect();
    let mut out = json!({
        "id": doc.id, "file": doc.file, "name": recipe.name, "revision": recipe.revision,
        "edited": recipe.edited, "version": doc.version, "mixamo": doc.mixamo, "base": base,
        "blanks": blanks, "changes": recipe.layers.iter().map(change_view).collect::<Vec<_>>(),
    });
    if recipe.version > 1 {
        out["format"] = json!(recipe.version);
        let clips: Vec<Value> = std::iter::once(clip_view(&recipe.base, &doc.base))
            .chain(
                recipe
                    .joined
                    .iter()
                    .zip(&doc.joined)
                    .map(|(clip, found)| clip_view(clip, found)),
            )
            .collect();
        out["clips"] = json!(clips);
        out["mirror"] = json!(recipe.mirror);
        out["time"] = to_short_value(&recipe.time);
        if let Some(refusal) = not_bakeable(doc) {
            out["bakeable"] = json!(false);
            out["notBakeable"] = json!(refusal);
        }
    }
    if !doc.warnings.is_empty() {
        out["warnings"] = json!(doc.warnings);
    }
    out
}

/// What to do next with a recipe: fill its blanks, add changes, or import
/// its base clip first.
pub(super) fn after(doc: &RecipeDoc) -> Vec<Value> {
    let mut next = Vec::new();
    if doc.base.status == BaseStatus::Missing
        || doc.joined.iter().any(|j| j.status == BaseStatus::Missing)
    {
        next.push(step(
            "import_animations",
            "Import the base clip into the library before the recipe can be baked.",
        ));
    }
    if doc.recipe.blanks.iter().any(|b| !b.filled()) {
        next.push(step(
            "update_recipe",
            "Fill the open blanks: a prop's asset id or a point.",
        ));
    } else if doc.recipe.layers.is_empty() {
        next.push(step(
            "update_recipe",
            "Add changes: reach, nudge, look-at or wave on a humanoid bone.",
        ));
    }
    next
}
