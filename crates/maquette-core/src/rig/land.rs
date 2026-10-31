//! What must hold before a draft becomes a version: the mesh and textures
//! match the version it came from and the rig check did not get worse.

use std::path::Path;

use crate::error::CoreResult;
use crate::inbox::analyse;

/// Why `draft` is worse than `source`, or `None` when it is not.
///
/// # Errors
///
/// Fails when either file is not a glTF model.
pub fn got_worse(source: &Path, draft: &Path) -> CoreResult<Option<String>> {
    let (a, b) = (analyse::analyse(source)?, analyse::analyse(draft)?);
    if a.triangles != b.triangles {
        return Ok(Some(format!(
            "Triangles differ from the source: {} and {}.",
            a.triangles, b.triangles
        )));
    }
    if a.textures.len() != b.textures.len() {
        return Ok(Some("Textures differ from the source.".to_string()));
    }
    let far = |r: &analyse::GlbReport| r.finger_weights.as_ref().map_or(0, |f| f.vertices);
    if far(&b) > far(&a) {
        return Ok(Some(format!(
            "More vertices on finger bones than the source: {} and {}.",
            far(&a),
            far(&b)
        )));
    }
    Ok(None)
}
