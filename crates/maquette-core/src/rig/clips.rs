//! Which clips a joint move would break: a clip with translation tracks on a
//! moved joint pulls it back where it was.

use std::collections::BTreeSet;
use std::path::Path;

use gltf::animation::Property;

use crate::error::{CoreError, CoreResult};

/// The clips of the GLB at `file` with translation tracks on `joints`.
///
/// # Errors
///
/// Fails when the file is not a glTF model.
pub fn clips_on_joints(file: &Path, joints: &[String]) -> CoreResult<Vec<String>> {
    let bytes = std::fs::read(file).map_err(|e| CoreError::io(file, e))?;
    let gltf = gltf::Gltf::from_slice_without_validation(&bytes)
        .map_err(|e| CoreError::Invalid(format!("This is not a glTF model. {e}")))?;
    let mut found = BTreeSet::new();
    for animation in gltf.document.animations() {
        let hits = animation.channels().any(|c| {
            c.target().property() == Property::Translation
                && c.target()
                    .node()
                    .name()
                    .is_some_and(|n| joints.iter().any(|j| j == n))
        });
        if hits {
            found.insert(animation.name().unwrap_or("clip").to_string());
        }
    }
    Ok(found.into_iter().collect())
}
