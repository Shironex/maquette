//! The scroll timeline: what the page does as it scrolls, on top of the
//! camera stops and the object states.
//!
//! The Timeline view lays the page out as one row per camera stop. A
//! prop's states change there through its stop and scroll triggers (see
//! [`super::states`]); this part adds what has no other home: how far a
//! model's parts spread apart at each stop. It lives in `layout.json` under
//! `timeline`, keyed by asset id like the placements:
//!
//! ```json
//! "timeline": {
//!   "explode": {
//!     "switch-cabinet": { "distance": 0.25, "easing": "ease-in-out",
//!       "keys": [{ "stop": "porch", "amount": 0 }, { "stop": "canopy", "amount": 1 }] }
//!   }
//! }
//! ```
//!
//! `amount` is 0 with the parts where the model file has them and 1 with
//! each part moved `distance` metres out from the model's middle, along the
//! line from that middle to the part's own. The page eases the amount from
//! one key to the next as it scrolls between their stops and holds it
//! before the first key and after the last. A build-up is the same track
//! run the other way: 1 first, then 0.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{Easing, MAX_STOPS, SceneLayout, invalid};
use crate::error::CoreResult;

/// The farthest a part may spread, in metres.
pub const MAX_SPREAD: f32 = 10.0;

/// How far a model's parts are spread apart at one camera stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExplodeKey {
    /// The camera stop's id.
    pub stop: String,
    /// 0 together, 1 spread the whole distance.
    pub amount: f32,
}

/// One model's explode or build-up track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExplodeTrack {
    /// Metres each part moves out at an amount of 1.
    pub distance: f32,
    /// How the amount eases from one key to the next.
    pub easing: Easing,
    /// At most one per stop, in any order; the page sorts them by stop.
    pub keys: Vec<ExplodeKey>,
}

impl Default for ExplodeTrack {
    fn default() -> Self {
        Self {
            distance: 0.25,
            easing: Easing::EaseInOut,
            keys: Vec::new(),
        }
    }
}

/// The Timeline view's own tracks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    /// Asset id to how its parts spread apart along the scroll.
    pub explode: BTreeMap<String, ExplodeTrack>,
}

fn check_track(asset: &str, track: &ExplodeTrack, stops: &BTreeSet<&str>) -> CoreResult<()> {
    if !(track.distance.is_finite() && track.distance > 0.0 && track.distance <= MAX_SPREAD) {
        return Err(invalid(format!(
            "The explode track of {asset} spreads its parts {} m. Use more than 0 and at most {MAX_SPREAD} m.",
            track.distance
        )));
    }
    if track.keys.len() > MAX_STOPS {
        return Err(invalid(format!(
            "The explode track of {asset} has more keys than there can be stops."
        )));
    }
    let mut seen = BTreeSet::new();
    for key in &track.keys {
        if !stops.contains(key.stop.as_str()) {
            return Err(invalid(format!(
                "The explode track of {asset} has a key at the camera stop {}, which is not in the layout.",
                key.stop
            )));
        }
        if !seen.insert(key.stop.as_str()) {
            return Err(invalid(format!(
                "The explode track of {asset} has two keys at the stop {}.",
                key.stop
            )));
        }
        if !(key.amount.is_finite() && (0.0..=1.0).contains(&key.amount)) {
            return Err(invalid(format!(
                "The explode track of {asset} has an amount of {} at {}. Use 0 to 1.",
                key.amount, key.stop
            )));
        }
    }
    Ok(())
}

/// Checks the timeline of `layout` against its camera stops.
pub(super) fn validate(layout: &SceneLayout) -> CoreResult<()> {
    let stops: BTreeSet<&str> = layout.stops.iter().map(|s| s.id.as_str()).collect();
    for (asset, track) in &layout.timeline.explode {
        if asset.trim().is_empty() {
            return Err(invalid("An explode track has no asset id."));
        }
        check_track(asset, track, &stops)?;
    }
    Ok(())
}

/// Drops the keys at camera stops the layout no longer has, so removing a
/// stop never leaves a key pointing at nothing. Returns how many went.
pub fn prune_stops(layout: &mut SceneLayout) -> usize {
    let stops: BTreeSet<String> = layout.stops.iter().map(|s| s.id.clone()).collect();
    let mut dropped = 0;
    for track in layout.timeline.explode.values_mut() {
        let before = track.keys.len();
        track.keys.retain(|k| stops.contains(&k.stop));
        dropped += before - track.keys.len();
    }
    dropped
}

#[cfg(test)]
mod tests;
