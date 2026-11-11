//! What an asset was before a built asset took its place, kept on its built
//! source in `built.json`:
//!
//! ```json
//! "replaced": {
//!   "at": "2026-11-10T09:30:00.000Z",
//!   "by": { "asset": "planner-built", "name": "Planner built", "index": 4,
//!           "trash": "planner-built-20261110-093000123" },
//!   "versions": [1, 2],
//!   "built": [3],
//!   "thumbnail": "thumbs/planner_v2.png",
//!   "origin": null, "status": "done", "stage": "web",
//!   "prep": { "size": { "axis": "longest", "value": 30, "unit": "cm" },
//!             "fixups": [{ "op": "decimate", "triangles": 60000 }],
//!             "skipBake": true, "padEdges": true, "parts": null },
//!   "materials": null
//! }
//! ```
//!
//! The asset keeps its id, so the layout, the camera stops and the ship data
//! never change. This record holds what did change (its versions, picture,
//! library origin, real size and fix-ups, material tweaks) and where the
//! built asset's own card went, so one undo puts the old model back as it
//! was. The replace itself is in core's `built::replace`.

use serde::{Deserialize, Serialize};

use crate::assets::{AssetOrigin, AssetStatus, Stage};
use crate::materials::ModelMaterials;
use crate::prepare::ModelPrep;

/// The built asset whose script and model took the place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplacedBy {
    /// Its id and name on the Board before it left.
    pub asset: String,
    pub name: String,
    /// Where its card stood among the project's assets, so an undo puts it
    /// back there.
    pub index: u32,
    /// Its id in the asset trash once it left the Board; an undo restores
    /// it from there. `None` when it could not be taken off.
    pub trash: Option<String>,
}

/// What an asset was before a built asset took its place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplacedModel {
    /// RFC 3339 UTC.
    pub at: String,
    pub by: ReplacedBy,
    /// The version numbers the asset had, in its order: the last one is the
    /// model an undo makes current again, and every version not in this
    /// list is taken off.
    pub versions: Vec<u32>,
    /// The versions a build of the script made since: the built model the
    /// replace copied in and every rebuild. A version taken off by an undo
    /// that is not listed here came from elsewhere (an Inbox accept, say).
    #[serde(default)]
    pub built: Vec<u32>,
    /// The card's picture, library origin, status and stage as they were.
    pub thumbnail: Option<String>,
    pub origin: Option<AssetOrigin>,
    pub status: AssetStatus,
    pub stage: Stage,
    /// Its real size, fix-ups and switches as `prepare.json` had them.
    pub prep: Option<ModelPrep>,
    /// Its material tweaks as `materials.json` had them.
    pub materials: Option<ModelMaterials>,
}

impl ReplacedModel {
    /// The version an undo makes current again.
    pub fn version(&self) -> Option<u32> {
        self.versions.last().copied()
    }
}

/// A replace undone after a Ship wrote the built model, kept in
/// `built.json` under `undone` by the asset's id: the next Ship puts the old
/// model back, so its approval names the swap back like a replace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoneReplace {
    /// RFC 3339 UTC, when it was undone.
    pub at: String,
    /// The script that stopped building the asset, relative to the project.
    pub script: String,
    /// The built version that left, and the file it shipped as.
    pub version: u32,
    pub shipped_as: Option<String>,
    pub triangles: Option<u32>,
    /// Bytes as a JS safe number.
    pub bytes: Option<f64>,
}

#[cfg(test)]
mod tests {
    use crate::built::BuiltFile;

    #[test]
    fn a_built_file_from_before_undone_and_built_still_reads() {
        let text = r#"{"version":1,"assets":{"planner":{"script":"built/planner.py",
            "helpers":[],"root":null,"last":null,"replaced":{"at":"2026-11-10T09:30:00.000Z",
            "by":{"asset":"planner-built","name":"Planner built","index":4,"trash":null},
            "versions":[1],"thumbnail":null,"origin":null,"status":"done","stage":"web",
            "prep":null,"materials":null}}}}"#;
        let file: BuiltFile = serde_json::from_str(text).expect("older built.json");
        assert!(file.undone.is_empty());
        let record = file.assets["planner"].replaced.as_ref().expect("record");
        assert!(record.built.is_empty(), "no version counts as built");
        // Nothing new is written for an empty undone list.
        let back = serde_json::to_value(&file).expect("json");
        assert!(back.get("undone").is_none());
    }
}
