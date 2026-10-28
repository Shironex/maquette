//! Lab jobs that work on one asset's refs: drawing the views it is
//! missing from its front ("Make the other views"), and a cleaned copy of
//! one ref ("Clean background").
//!
//! Both land in a direction of their own that records the asset
//! ([`RefsJob`]): "<asset> views", one image per candidate titled by its
//! view (`left`), and "<asset> clean", one image per copy titled by the
//! ref it cleans. Nothing reaches the asset's refs until a candidate is
//! used (`crate::refs::use_view_image`, `crate::refs::use_clean_copy`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::codex::assemble;
use super::style::StyleBoard;
use super::{Aspect, Direction, NewConcepts, load_direction, modify_direction};
use crate::assets::Asset;
use crate::error::{CoreError, CoreResult};
use crate::project::slugify;
use crate::refs::{AssetRef, RefView};

/// The most candidates one view gets in one go.
pub const MAX_PER_VIEW: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum RefsJobKind {
    Views,
    Clean,
}

/// What a direction of refs work is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RefsJob {
    pub asset: String,
    pub kind: RefsJobKind,
}

/// What "Make the other views" asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewOtherViews {
    pub project: String,
    pub asset: String,
    /// The views to draw. Empty draws every empty slot; naming a filled
    /// one draws it again. The front is never drawn.
    #[serde(default)]
    pub views: Vec<RefView>,
    /// Candidates per view, 1 to 4.
    pub count: u32,
    #[serde(default = "super::style_on")]
    pub use_style: bool,
}

/// The background a clean copy gets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum CleanGround {
    /// The grey of object refs and Board renders.
    #[default]
    StudioGrey,
    Transparent,
}

/// What "Clean background" asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewCleanRef {
    pub project: String,
    pub asset: String,
    /// The ref to clean, in `refs/`.
    pub file: String,
    #[serde(default)]
    pub ground: CleanGround,
    /// Center the object and pad the image to a square.
    #[serde(default)]
    pub pad: bool,
    /// Draw what the frame cut off. Off keeps it true to the original.
    #[serde(default)]
    pub extend: bool,
}

/// The views a request draws, front excluded, in slot order: the named
/// ones, else the slots `refs` leaves empty.
pub fn views_to_draw(refs: &[AssetRef], asked: &[RefView]) -> CoreResult<Vec<RefView>> {
    if asked.contains(&RefView::Front) || asked.contains(&RefView::Other) {
        return Err(CoreError::Invalid(
            "Only left, back and right are drawn; the front is what they are drawn from."
                .to_string(),
        ));
    }
    let wanted: Vec<RefView> = RefView::SLOTS[1..]
        .iter()
        .copied()
        .filter(|v| {
            if asked.is_empty() {
                !refs.iter().any(|r| r.view == *v)
            } else {
                asked.contains(v)
            }
        })
        .collect();
    if wanted.is_empty() {
        return Err(CoreError::Invalid(
            "Every view is filled. Name a view to draw it again.".to_string(),
        ));
    }
    Ok(wanted)
}

/// The asset's front ref.
pub fn front_of(refs: &[AssetRef]) -> CoreResult<String> {
    refs.iter()
        .find(|r| r.view == RefView::Front)
        .map(|r| r.file.clone())
        .ok_or_else(|| {
            CoreError::Invalid("Tag a front first: the other views are drawn from it.".to_string())
        })
}

fn side_words(view: RefView) -> &'static str {
    match view {
        RefView::Left => "its left side, the camera turned 90 degrees to the object's left",
        RefView::Back => "behind it, the camera turned 180 degrees",
        RefView::Right => "its right side, the camera turned 90 degrees to the object's right",
        RefView::Front | RefView::Other => "the front",
    }
}

/// The prompt for one view candidate. The front is the first attached
/// image; a style board's images come after it.
pub fn view_prompt(asset: &Asset, view: RefView, out: &Path, style: Option<&StyleBoard>) -> String {
    let lead = format!(
        "Draw the {name} from {side}. The first attached image is its front view: \
         this is the very same {name}, turned to that side.\n\n\
         Keep the front image's scale, framing, light and plain background, the object centered \
         and whole in frame. Show no parts the front does not show or imply, and keep its \
         materials, colours and proportions. No other objects, no text, no labels, no watermark. \
         Use a 1:1 square composition.",
        name = asset.name.trim(),
        side = side_words(view),
    );
    assemble(&lead, style, out)
}

/// The prompt for a clean copy of one ref, the first attached image.
pub fn clean_prompt(new: &NewCleanRef, asset: &Asset, out: &Path) -> String {
    let ground = match new.ground {
        CleanGround::StudioGrey => {
            "a plain light studio grey background, even soft light and no cast shadow on it"
        }
        CleanGround::Transparent => "a fully transparent background (a PNG with alpha)",
    };
    let pad = if new.pad {
        " Center the object and pad the image to a 1:1 square, with room around it on every side."
    } else {
        " Keep the object where it is and the image's own size and shape."
    };
    let extend = if new.extend {
        " Where the frame cuts the object off, draw the missing part so the whole object shows."
    } else {
        " Where the frame cuts the object off, leave it cut off: invent nothing."
    };
    let lead = format!(
        "Clean up the attached image, a reference of the {name} for a 3D model. Keep the {name} \
         exactly as it is (shape, materials, colours, details) and remove everything else: other \
         objects, clutter, text, frames and the old background. Put it on {ground}.{pad}{extend}",
        name = asset.name.trim(),
    );
    assemble(&lead, None, out)
}

/// The direction for an asset's refs work: "<asset> views" or "<asset>
/// clean", numbered when another asset already has that name.
fn direction_name(project_dir: &Path, asset: &Asset, kind: RefsJobKind) -> CoreResult<String> {
    let word = match kind {
        RefsJobKind::Views => "views",
        RefsJobKind::Clean => "clean",
    };
    let base: String = asset.name.trim().chars().take(48).collect();
    for n in 1..=99 {
        let name = if n == 1 {
            format!("{base} {word}")
        } else {
            format!("{base} {word} {n}")
        };
        match load_direction(project_dir, &slugify(&name))? {
            None => return Ok(name),
            Some(d)
                if d.refs_job
                    .as_ref()
                    .is_some_and(|j| j.asset == asset.id && j.kind == kind) =>
            {
                return Ok(name);
            }
            Some(_) => {}
        }
    }
    Err(CoreError::Invalid(format!(
        "There are too many directions named after {}.",
        asset.name
    )))
}

/// One image a refs job will make: where it goes and its title (the view
/// name, or the ref it cleans).
pub struct PlannedRef {
    pub index: u32,
    pub file: String,
    pub title: String,
}

/// Creates (or adds to) the asset's refs direction and plans one image per
/// title, attaching `source` as its reference.
pub(crate) fn prepare(
    project_dir: &Path,
    asset: &Asset,
    kind: RefsJobKind,
    source: &str,
    titles: &[String],
    style_revision: Option<u32>,
) -> CoreResult<(Direction, Vec<PlannedRef>)> {
    let name = direction_name(project_dir, asset, kind)?;
    #[expect(clippy::cast_possible_truncation, reason = "at most 12 images")]
    let count = titles.len() as u32;
    let brief = NewConcepts {
        project: String::new(),
        direction: name,
        prompt: match kind {
            RefsJobKind::Views => format!("Other views of {} from its front.", asset.name),
            RefsJobKind::Clean => format!("A clean background for a ref of {}.", asset.name),
        },
        refs: vec![source.to_string()],
        aspect: Aspect::Square,
        count,
        use_style: style_revision.is_some(),
    };
    let (direction, files) = super::prepare_with(project_dir, &brief, style_revision)?;
    let (direction, ()) = modify_direction(project_dir, &direction.id, |d| {
        d.refs_job = Some(RefsJob {
            asset: asset.id.clone(),
            kind,
        });
        Ok(())
    })?;
    let planned = files
        .into_iter()
        .zip(titles)
        .map(|((index, file), title)| PlannedRef {
            index,
            file,
            title: title.clone(),
        })
        .collect();
    Ok((direction, planned))
}

#[cfg(test)]
mod tests;
