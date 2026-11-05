//! How a recipe bake's clip is named against the model's clips: every
//! clip an earlier bake of the recipe added is replaced, whatever it was
//! named; any other stays and the new clip takes the first free name.

/// How a bake's clip is named against the model's clips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipName {
    /// The names the new clip must keep clear of.
    pub taken: Vec<String>,
    /// The model's clips earlier bakes of this recipe added: the new one
    /// replaces them all, so rebakes never pile up `name_2`, `name_3`.
    pub replace: Vec<String>,
}

/// The rule for a recipe's clip on a model whose clips are `clips` and
/// whose accepted recipe bakes are `bakes`: each clip that an earlier bake
/// of recipe `recipe` added is replaced and its name is free again; any
/// other keeps its name, and the new clip takes the base name or `base_2`
/// and on.
pub(crate) fn clip_name(
    clips: &[String],
    bakes: &[crate::recipes::baked::RecipeBakeUse],
    recipe: &str,
) -> ClipName {
    let ours = |clip: &String| bakes.iter().any(|b| b.recipe == recipe && &b.clip == clip);
    let (replace, taken) = clips.iter().cloned().partition(|c| ours(c));
    ClipName { taken, replace }
}

/// The sentence the review shows about the clip's name, if it clashed.
pub(super) fn clip_note(base: &str, name: &str, replace: &[String], model: &str) -> Option<String> {
    let replaced = match replace {
        [] => None,
        [old] => Some(format!(
            "Replaces {old}, the clip an earlier bake of this recipe added to {model}."
        )),
        _ => Some(format!(
            "Replaces {}, the clips earlier bakes of this recipe added to {model}.",
            replace.join(", ")
        )),
    };
    let kept = (!base.is_empty() && name != base).then(|| {
        format!(
            "{model} already has a clip named {base} that this recipe did not make, so it stays and this one is named {name}."
        )
    });
    match (replaced, kept) {
        (Some(r), Some(k)) => Some(format!("{r} {k}")),
        (r, k) => r.or(k),
    }
}
