//! Paths for agents: `set_layout` adds, changes and removes the curves
//! through the scene and sets their energize tracks through the edits here;
//! `get_layout` returns them with the layout.

use std::collections::BTreeMap;

use maquette_core::layout::{PathCurve, PathKey, PathPatch, PathTrack};
use rmcp::schemars;
use serde::Deserialize;

use super::explicit;
use super::motion::EasingEdit;

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CurveEdit {
    /// Straight runs from point to point.
    Polyline,
    /// A smooth curve through every point.
    CatmullRom,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathEdit {
    /// An existing path id changes that path. Leave it out (or give a new one) to add a path,
    /// which then needs name and points.
    pub id: Option<String>,
    pub name: Option<String>,
    /// How the points join; catmull-rom when a new path leaves it out.
    pub curve: Option<CurveEdit>,
    /// Every point in order from the start, in metres with y up: 2 to 128 of them.
    pub points: Option<Vec<[f32; 3]>>,
    /// The asset whose placement the first point follows from now on. null lets it go.
    #[serde(default, deserialize_with = "explicit")]
    pub start: Option<Option<String>>,
    /// The asset whose placement the last point follows from now on. null lets it go.
    #[serde(default, deserialize_with = "explicit")]
    pub end: Option<Option<String>>,
    /// How thick the tube is, in metres; 0.035 when a new path leaves it out.
    pub width: Option<f32>,
    /// The tube's colour as #rrggbb.
    pub color: Option<String>,
    /// The colour the lit part glows, as #rrggbb.
    pub glow: Option<String>,
    /// Remove the path with this id, and its energize track.
    #[serde(default)]
    pub remove: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnergizeKeyEdit {
    /// The camera stop id, as get_layout lists it.
    pub stop: String,
    /// 0 unlit, 1 lit from the first point to the last.
    pub amount: f32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnergizeEdit {
    /// The path id.
    pub path: String,
    /// How the amount eases from one key to the next; linear when left out.
    pub easing: Option<EasingEdit>,
    /// How much is lit at camera stops, at most one key per stop. Replaces the track's keys.
    #[serde(default)]
    pub keys: Vec<EnergizeKeyEdit>,
    /// Drop the path's energize track, so it stays unlit.
    #[serde(default)]
    pub remove: bool,
}

/// The layout patch's path changes, in order.
pub(super) fn paths_patch(edits: &[PathEdit]) -> Vec<PathPatch> {
    edits
        .iter()
        .map(|e| PathPatch {
            id: e.id.as_ref().map(|id| id.trim().to_string()),
            name: e.name.clone(),
            curve: e.curve.map(|c| match c {
                CurveEdit::Polyline => PathCurve::Polyline,
                CurveEdit::CatmullRom => PathCurve::CatmullRom,
            }),
            points: e.points.clone(),
            start: e
                .start
                .as_ref()
                .map(|a| a.as_ref().map(|s| s.trim().to_string())),
            end: e
                .end
                .as_ref()
                .map(|a| a.as_ref().map(|s| s.trim().to_string())),
            width: e.width,
            color: e.color.clone(),
            glow: e.glow.clone(),
            remove: e.remove,
        })
        .collect()
}

/// The layout patch's energize tracks: each path's whole track, or `None`
/// to drop it.
pub(super) fn energize_patch(edits: &[EnergizeEdit]) -> BTreeMap<String, Option<PathTrack>> {
    edits
        .iter()
        .map(|e| {
            let track = (!e.remove).then(|| PathTrack {
                easing: e
                    .easing
                    .map_or_else(|| PathTrack::default().easing, EasingEdit::easing),
                keys: e
                    .keys
                    .iter()
                    .map(|k| PathKey {
                        stop: k.stop.trim().to_string(),
                        amount: k.amount,
                    })
                    .collect(),
            });
            (e.path.trim().to_string(), track)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use maquette_core::layout::Easing;

    #[test]
    fn edits_trim_ids_and_keep_explicit_nulls() {
        let edits: Vec<PathEdit> = serde_json::from_value(serde_json::json!([
            { "id": " feed ", "end": null, "curve": "polyline" },
            { "name": "Earth", "points": [[0, 0, 0], [1, 0, 0]], "start": " lamp " }
        ]))
        .expect("edits");
        let patch = paths_patch(&edits);
        assert_eq!(patch[0].id.as_deref(), Some("feed"));
        assert_eq!(patch[0].end, Some(None));
        assert_eq!(patch[0].start, None);
        assert_eq!(patch[0].curve, Some(PathCurve::Polyline));
        assert_eq!(patch[1].start, Some(Some("lamp".into())));

        let energize: Vec<EnergizeEdit> = serde_json::from_value(serde_json::json!([
            { "path": "feed", "keys": [{ "stop": " porch ", "amount": 1 }] },
            { "path": "earth", "remove": true }
        ]))
        .expect("energize");
        let tracks = energize_patch(&energize);
        let feed = tracks["feed"].as_ref().expect("feed");
        assert_eq!(feed.easing, Easing::Linear);
        assert_eq!(feed.keys[0].stop, "porch");
        assert!(tracks["earth"].is_none());
    }
}
