//! The settings table of a replace: what the asset's `prepare.json` entry
//! and material tweaks become, setting by setting.
//!
//! | Recorded for the old model                 | What the replace does                   |
//! | ------------------------------------------ | --------------------------------------- |
//! | Real size                                  | carried                                 |
//! | Fix-ups (rotate, apply transforms, remove  | reset: the built asset's own take their |
//! | loose, merge, split, decimate, shrink      | place                                   |
//! | textures, origin to bottom)                |                                         |
//! | Skip the bake, pad texture edges           | reset to the built asset's own choice   |
//! | Part names, joins and hidden parts         | carried where the part is there, else   |
//! |                                            | reset                                   |
//! | Material tweaks                            | reset: the built asset's own take their |
//! |                                            | place                                   |
//! | Compression presets (per model file)       | not carried, reported                   |
//! | Library origin                             | reset                                   |
//!
//! The real size is a measure ("30 cm on its longest side"), not a factor:
//! Build scene scales the new model to it from the new model's own bounds,
//! so it stands at the size the old one stood and is never scaled twice.
//! Everything reset is kept on the built source's `replaced` record and
//! comes back with an undo.

use crate::assets::AssetOrigin;
use crate::materials::{MaterialTweak, ModelMaterials};
use crate::pipeline::CompressPreset;
use crate::prepare::{Fixup, FixupKind, ModelPrep, PartsPlan};

use super::report::{SwapAction, SwapSetting};

/// The asset's preparation and material tweaks once replaced, and the lines
/// that say how they got there.
pub(super) struct Merged {
    pub prep: ModelPrep,
    pub looks: Option<ModelMaterials>,
    pub lines: Vec<SwapSetting>,
}

fn line(what: impl Into<String>, action: SwapAction, why: &str) -> SwapSetting {
    SwapSetting::new(what, action, why)
}

const FROM_BUILT: &str = "It is the built asset's own, set for this model.";

fn reset_why(kind: FixupKind) -> &'static str {
    match kind {
        FixupKind::Rotate => {
            "It turned the old model. The built model is not turned: build it facing the way the old one stood."
        }
        FixupKind::OriginBottom => {
            "It moved the old model's origin to its bottom centre. The built model is not moved: build it around that point."
        }
        _ => "It repaired the old model's mesh or textures.",
    }
}

fn size_lines(old: &ModelPrep, own: &ModelPrep, scale: f32, lines: &mut Vec<SwapSetting>) {
    match (&old.size, &own.size) {
        (Some(size), theirs) => {
            lines.push(line(
                format!("Real size, {}", size.describe()),
                SwapAction::Carried,
                &format!(
                    "It is a measure, so the built model is scaled to it from its own bounds (by {} here) and never twice.",
                    crate::prepare::trim_number(scale)
                ),
            ));
            if let Some(theirs) = theirs.as_ref().filter(|t| *t != size) {
                lines.push(line(
                    format!("Real size of the built asset, {}", theirs.describe()),
                    SwapAction::Reset,
                    "The asset keeps the real size it stood at.",
                ));
            }
        }
        (None, Some(theirs)) => lines.push(line(
            format!("Real size, {}", theirs.describe()),
            SwapAction::Brought,
            FROM_BUILT,
        )),
        (None, None) => {}
    }
}

fn fixup_lines(old: &[Fixup], own: &[Fixup], lines: &mut Vec<SwapSetting>) {
    for fixup in old {
        lines.push(line(
            fixup.describe(),
            SwapAction::Reset,
            reset_why(fixup.kind()),
        ));
    }
    for fixup in own {
        lines.push(line(fixup.describe(), SwapAction::Brought, FROM_BUILT));
    }
}

fn switch_lines(old: &ModelPrep, own: &ModelPrep, lines: &mut Vec<SwapSetting>) {
    if old.skips_bake() && !own.skips_bake() {
        lines.push(line(
            "Skips the bake",
            SwapAction::Reset,
            "A generated scan skips the bake. A built model bakes like every other built piece.",
        ));
    }
    if own.skips_bake() {
        lines.push(line("Skips the bake", SwapAction::Brought, FROM_BUILT));
    }
    if old.pads_edges() && !own.pads_edges() {
        lines.push(line(
            "Pads texture edges",
            SwapAction::Reset,
            "It fills the gaps of a scan's texture atlas.",
        ));
    }
    if own.pads_edges() {
        lines.push(line("Pads texture edges", SwapAction::Brought, FROM_BUILT));
    }
}

/// The old plan's names, joins and hidden parts that the new model still
/// has, with a line for each one either way.
fn carried_parts(old: &PartsPlan, parts: &[String], lines: &mut Vec<SwapSetting>) -> PartsPlan {
    let has = |part: &String| parts.contains(part);
    let mut next = PartsPlan::default();
    for (from, to) in &old.names {
        let what = format!("Part name {from} as {to}");
        if has(from) {
            next.names.insert(from.clone(), to.clone());
            lines.push(line(
                what,
                SwapAction::Carried,
                "The built model has that part.",
            ));
        } else {
            lines.push(line(
                what,
                SwapAction::Reset,
                "The built model has no part of that name.",
            ));
        }
    }
    for group in &old.merge {
        let what = format!("Join of {}", group.join(", "));
        if group.iter().all(has) {
            next.merge.push(group.clone());
            lines.push(line(
                what,
                SwapAction::Carried,
                "The built model has those parts.",
            ));
        } else {
            lines.push(line(
                what,
                SwapAction::Reset,
                "The built model does not have all of those parts.",
            ));
        }
    }
    for part in &old.hidden {
        if has(part) {
            next.hidden.push(part.clone());
        } else {
            lines.push(line(
                format!("Hidden part {part}"),
                SwapAction::Reset,
                "The built model has no part of that name.",
            ));
        }
    }
    next.normalized()
}

/// The built asset's own part names when it has some, else what the old
/// plan still names in the new model.
fn parts_plan(
    old: Option<&PartsPlan>,
    own: Option<&PartsPlan>,
    parts: &[String],
    lines: &mut Vec<SwapSetting>,
) -> Option<PartsPlan> {
    let own = own.filter(|p| !p.is_empty());
    let next = match (old.filter(|p| !p.is_empty()), own) {
        (old, Some(own)) => {
            lines.push(line(
                "Part names and joins of the built asset",
                SwapAction::Brought,
                FROM_BUILT,
            ));
            if old.is_some() {
                lines.push(line(
                    "Part names and joins of the old model",
                    SwapAction::Reset,
                    "The built asset brings its own.",
                ));
            }
            own.normalized()
        }
        (Some(old), None) => carried_parts(old, parts, lines),
        (None, None) => PartsPlan::default(),
    };
    (!next.is_empty()).then_some(next)
}

fn tweak_words(tweak: &MaterialTweak) -> String {
    let mut words = Vec::new();
    if let Some(tint) = &tweak.tint {
        words.push(format!("tint {tint}"));
    }
    if tweak.roughness.is_some() {
        words.push("roughness".to_string());
    }
    if tweak.metalness.is_some() {
        words.push("metalness".to_string());
    }
    if tweak.emissive_strength.is_some() {
        words.push("emissive strength".to_string());
    }
    words.join(", ")
}

fn look_lines(look: &ModelMaterials, action: SwapAction, why: &str) -> Vec<SwapSetting> {
    let mut out = Vec::new();
    if !look.all.is_empty() {
        out.push(line(
            format!(
                "Material tweak for every material ({})",
                tweak_words(&look.all)
            ),
            action,
            why,
        ));
    }
    for (name, tweak) in look.materials.iter().filter(|(_, t)| !t.is_empty()) {
        out.push(line(
            format!("Material tweak for {name} ({})", tweak_words(tweak)),
            action,
            why,
        ));
    }
    out
}

/// What the asset's settings become when the built asset's model takes the
/// place. `scale` is what the carried real size gives the new model and
/// `parts` are the parts the new model's file has.
pub(super) fn merge(
    old: Option<&ModelPrep>,
    own: Option<&ModelPrep>,
    looks: (Option<&ModelMaterials>, Option<&ModelMaterials>),
    parts: &[String],
    scale: f32,
) -> Merged {
    let empty = ModelPrep::default();
    let (old, own) = (old.unwrap_or(&empty), own.unwrap_or(&empty));
    let mut lines = Vec::new();
    size_lines(old, own, scale, &mut lines);
    fixup_lines(&old.fixups, &own.fixups, &mut lines);
    switch_lines(old, own, &mut lines);
    let parts = parts_plan(old.parts.as_ref(), own.parts.as_ref(), parts, &mut lines);
    let (old_looks, own_looks) = looks;
    if let Some(look) = old_looks {
        lines.extend(look_lines(
            look,
            SwapAction::Reset,
            "It corrected the old model's look. The script sets the built model's.",
        ));
    }
    let looks = own_looks.filter(|l| !l.is_empty()).cloned();
    if let Some(look) = &looks {
        lines.extend(look_lines(look, SwapAction::Brought, FROM_BUILT));
    }
    Merged {
        prep: next_prep(old, own, parts),
        looks,
        lines,
    }
}

/// The entry the asset gets: its own real size (else the built asset's)
/// over the built asset's fix-ups and switches.
pub(super) fn next_prep(old: &ModelPrep, own: &ModelPrep, parts: Option<PartsPlan>) -> ModelPrep {
    ModelPrep {
        size: old.size.or(own.size),
        fixups: own.fixups.clone(),
        skip_bake: own.skip_bake,
        pad_edges: own.pad_edges,
        parts,
    }
}

fn preset_words(preset: &CompressPreset) -> String {
    format!(
        "{} px, {:?}, meshopt {:?}",
        preset.texture_size, preset.codec, preset.meshopt
    )
    .to_lowercase()
}

/// The compression presets of the two assets' model files. A preset belongs
/// to one model file, so none follows to the new file, except a locked
/// character row, which follows its asset's newest model.
pub(super) fn preset_lines(
    presets: &[CompressPreset],
    old_stems: &[String],
    own_stems: &[String],
    new_stem: &str,
) -> (Vec<SwapSetting>, Vec<String>) {
    let (mut lines, mut warnings) = (Vec::new(), Vec::new());
    let of = |stems: &[String]| -> Vec<&CompressPreset> {
        presets
            .iter()
            .filter(|p| stems.iter().any(|s| s.eq_ignore_ascii_case(&p.model)))
            .collect()
    };
    for preset in of(old_stems) {
        let what = format!(
            "Compression preset of {} ({})",
            preset.model,
            preset_words(preset)
        );
        if let Some(lock) = &preset.lock {
            warnings.push(format!(
                "The locked compression preset of {} follows the asset's newest model, so it now applies to the built model ({lock}). Edit pipeline.json if the built model needs another.",
                preset.model
            ));
            lines.push(line(
                what,
                SwapAction::Carried,
                "A locked row follows its asset's newest model.",
            ));
        } else {
            lines.push(line(
                what,
                SwapAction::Reset,
                &format!(
                    "A preset belongs to one model file. {new_stem} starts on the workspace defaults; it comes back into use with an undo."
                ),
            ));
        }
    }
    for preset in of(own_stems) {
        lines.push(line(
            format!(
                "Compression preset of {} ({})",
                preset.model,
                preset_words(preset)
            ),
            SwapAction::Reset,
            &format!(
                "A preset belongs to one model file. Set it again for {new_stem} if the built model needs it."
            ),
        ));
    }
    (lines, warnings)
}

/// The library origin goes: the model no longer comes from that entry.
pub(super) fn origin_line(origin: Option<&AssetOrigin>) -> Option<SwapSetting> {
    origin.map(|o| {
        line(
            format!("Library origin, {} v{}", o.entry, o.version),
            SwapAction::Reset,
            "The model no longer comes from that library entry, so no update from it is offered.",
        )
    })
}
