//! A person's save from an open canvas: the operations between the canvas
//! the view last had from the file and what it shows now, laid over the
//! file as it is. The view also sends the `created` time of the canvas it
//! opened, so a save never lands on another canvas that took the same id
//! meanwhile (an older canvas moved aside to make room for the camera plan,
//! say): the operations would be laid over the wrong canvas.

use std::path::Path;

use super::PlanFile;
use super::change::{Changed, Writer, change};
use super::ops::PlanOp;
use crate::error::{CoreError, CoreResult};

/// Lays a person's `ops` over the canvas `id`, refused with nothing
/// written when the file is no longer the canvas made at `created`.
///
/// # Errors
///
/// Refuses a replaced canvas; fails as [`change`].
pub fn apply_person(
    project_dir: &Path,
    id: &str,
    created: &str,
    ops: Vec<PlanOp>,
) -> CoreResult<Changed> {
    change(project_dir, id, &Writer::PERSON, |current| {
        same_canvas(current, Some(created))?;
        Ok(ops)
    })
}

/// Refuses a person's change to a canvas that is not the one they opened:
/// `created` is the opened canvas's, so a change never lands on another
/// canvas that took the id. `None` skips the check (tests and tools that
/// open no canvas).
///
/// # Errors
///
/// Fails when `created` is given and is not the canvas's.
pub fn same_canvas(plan: &PlanFile, created: Option<&str>) -> CoreResult<()> {
    match created {
        Some(created) if plan.created != created => Err(CoreError::Invalid(
            "This canvas was replaced on disk since it was opened (it may have moved to make room for the camera plan), so your edits were not saved. Open it again from the Plan list; copy anything you need first.".into(),
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plans::{NodeKind, PlanNode, PlanPreset, create, get};

    #[test]
    fn a_save_meant_for_a_replaced_canvas_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let made = create(dir.path(), "Night story", PlanPreset::Story).expect("create");
        let note = PlanNode {
            id: "n-1".into(),
            kind: NodeKind::Note,
            x: 0,
            y: 0,
            width: None,
            title: "Bells".into(),
            text: String::new(),
            image: None,
            url: None,
            asset: None,
            direction: None,
            by: None,
            person_wrote: false,
            stop: None,
        };
        let op = PlanOp::AddNode { node: note };
        let refused = apply_person(
            dir.path(),
            &made.id,
            "2001-01-01T00:00:00Z",
            vec![op.clone()],
        )
        .expect_err("replaced");
        assert!(
            refused.to_string().contains("replaced on disk"),
            "{refused}"
        );
        assert!(
            get(dir.path(), &made.id)
                .expect("get")
                .plan
                .nodes
                .is_empty()
        );
        apply_person(dir.path(), &made.id, &made.plan.created, vec![op]).expect("save");
        assert_eq!(get(dir.path(), &made.id).expect("get").plan.nodes.len(), 1);
    }
}
