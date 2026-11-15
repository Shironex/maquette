//! A foot pin across a seam into a clip that travels: the root carries the
//! body away while the pin holds the foot, so the foot drags and the bake
//! review reports the drift. This reads the clip library's root motion to
//! say so when the pin is written, before any bake.

use crate::recipes::RecipeFile;
use crate::recipes::pins::{is_foot, spans};
use crate::recipes::timeline::{ClipSpan, timeline};

use super::{AnimationClip, match_base};

/// How far a clip may carry the root over the frames a foot pin covers
/// before the pin is worth a warning, metres.
pub const PIN_TRAVEL_WARN_M: f32 = 0.05;

/// About how far clip `span` carries the root over frames `first` to
/// `last` of the timeline, from its whole root motion; none when it does
/// not travel.
fn travel(clip: &AnimationClip, span: &ClipSpan, first: u32, last: u32) -> Option<f32> {
    let motion = clip.root_motion.as_ref()?;
    let from = first.max(span.first);
    let to = last.min(span.last);
    let length = span.last.saturating_sub(span.first).max(1);
    #[allow(clippy::cast_precision_loss)]
    let share = to.saturating_sub(from) as f32 / length as f32;
    Some(motion.metres * share)
}

/// One line per foot pin that spans a seam and covers a clip that carries
/// the root at least [`PIN_TRAVEL_WARN_M`] over the pinned frames. Clips the
/// library does not have are skipped.
pub fn pin_seam_warnings(recipe: &RecipeFile, clips: &[AnimationClip]) -> Vec<String> {
    let t = timeline(recipe);
    let bases: Vec<_> = recipe.clips().collect();
    let mut out = Vec::new();
    for pin in spans(recipe, &t).iter().filter(|p| is_foot(&p.bone)) {
        let covered: Vec<&ClipSpan> = t
            .clips
            .iter()
            .filter(|c| c.first <= pin.last && pin.first <= c.last)
            .collect();
        let Some(crossed) = covered.get(1) else {
            continue;
        };
        let seam = t
            .seams
            .iter()
            .find(|s| s.clip == crossed.clip)
            .map_or(crossed.first, |s| s.frame);
        let moving = covered.iter().find_map(|span| {
            let base = bases.get(span.clip as usize)?;
            let (_, clip) = match_base(clips, base);
            let clip = clip?;
            let metres = travel(clip, span, pin.first, pin.last)?;
            let direction = clip.root_motion.as_ref().map(|m| m.direction.clone());
            (metres >= PIN_TRAVEL_WARN_M).then(|| (base.name.clone(), metres, direction))
        });
        if let Some((name, metres, direction)) = moving {
            let way = direction.map(|d| format!(" {d}")).unwrap_or_default();
            out.push(format!(
                "The pin {} holds {} on frames {} to {}, across the seam at frame {seam}, over {name}, which carries the root about {metres:.2} m{way} in those frames: the planted foot will drag and the bake review will show the drift. End the pin before frame {seam}, start it after, or join a clip that stays in place.",
                pin.layer, pin.bone, pin.first, pin.last
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::library_clip;
    use super::super::base_of;
    use super::*;
    use crate::animations::RootMotion;
    use crate::recipes::{LayerKind, LayerSettings, RecipeLayer};

    fn foot_pin(start: f32, end: f32) -> RecipeLayer {
        RecipeLayer::new(
            "l-1",
            LayerKind::Pin,
            "leftFoot",
            LayerSettings {
                start,
                end,
                ..LayerSettings::default()
            },
        )
    }

    /// An idle, then a run that carries the root 3.6 m forward.
    fn idle_then_run() -> (RecipeFile, Vec<AnimationClip>) {
        let idle = library_clip("idle", "Standing Idle", "Singles", "h1");
        let mut run = library_clip("run", "Running", "Singles", "h2");
        run.root_motion = Some(RootMotion {
            metres: 3.6,
            direction: "forward".into(),
        });
        let mut recipe = RecipeFile::new("Idle to run", base_of(&idle));
        recipe.joined = vec![base_of(&run)];
        (recipe, vec![idle, run])
    }

    #[test]
    fn a_foot_pin_across_the_seam_into_a_travelling_clip_warns() {
        let (mut recipe, clips) = idle_then_run();
        recipe.layers = vec![foot_pin(0.2, 0.8)];
        let lines = pin_seam_warnings(&recipe, &clips);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].contains("Running") && lines[0].contains("forward"),
            "{}",
            lines[0]
        );
    }

    #[test]
    fn a_pin_before_the_seam_or_on_a_clip_that_stays_put_says_nothing() {
        let (mut recipe, clips) = idle_then_run();
        recipe.layers = vec![foot_pin(0.05, 0.3)];
        assert!(pin_seam_warnings(&recipe, &clips).is_empty());

        let mut still = clips.clone();
        still[1].root_motion = None;
        recipe.layers = vec![foot_pin(0.2, 0.8)];
        assert!(pin_seam_warnings(&recipe, &still).is_empty());
    }
}
