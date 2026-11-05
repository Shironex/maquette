//! How a recipe bake's clip is named against the model's clips: a same
//! named clip an earlier bake of the recipe added is replaced, any other
//! stays and the new clip takes the next free name.

/// How a bake's clip is named against the model's clips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipName {
    /// The names the new clip must keep clear of.
    pub taken: Vec<String>,
    /// The clip of the same name an earlier bake of this recipe added: the
    /// new one replaces it and keeps the name.
    pub replace: Option<String>,
}

/// The rule for a recipe's clip named `base` on a model whose clips are
/// `clips` and whose accepted recipe bakes are `bakes`: a same named clip
/// that an earlier bake of recipe `recipe` added is replaced; any other
/// keeps its name, and the new clip takes `base_2` and on.
pub(crate) fn clip_name(
    base: &str,
    clips: &[String],
    bakes: &[crate::recipes::baked::RecipeBakeUse],
    recipe: &str,
) -> ClipName {
    let ours = !base.is_empty()
        && clips.iter().any(|c| c == base)
        && bakes.iter().any(|b| b.recipe == recipe && b.clip == base);
    if !ours {
        return ClipName {
            taken: clips.to_vec(),
            replace: None,
        };
    }
    ClipName {
        taken: clips.iter().filter(|c| *c != base).cloned().collect(),
        replace: Some(base.to_string()),
    }
}

/// The sentence the review shows about the clip's name, if it clashed.
pub(super) fn clip_note(
    base: &str,
    name: &str,
    replace: Option<&str>,
    model: &str,
) -> Option<String> {
    if let Some(old) = replace {
        return Some(format!(
            "Replaces {old}, the clip an earlier bake of this recipe added to {model}."
        ));
    }
    (!base.is_empty() && name != base).then(|| {
        format!(
            "{model} already has a clip named {base} that this recipe did not make, so it stays and this one is named {name}."
        )
    })
}
