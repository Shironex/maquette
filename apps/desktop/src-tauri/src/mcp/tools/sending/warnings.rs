//! The warnings a send reports, the same checks the Send to 3D panel shows:
//! the weak refs, and the note that the asset has a model already. They
//! never stop a send.

use maquette_core::providers::ProviderJob;
use maquette_core::refs::WarningKind;
use serde_json::{Value, json};

use super::super::to_value;
use crate::commands::refs::{WarningsItem, warnings};
use crate::mcp::host::Host;

/// What a job is warned about: the weak refs as a JSON list and, when the
/// asset has a model already, the note that this send makes its next version
/// (null otherwise).
pub(super) struct Warned {
    pub refs: Value,
    pub model: Value,
}

/// The warnings for the images a job went with, as a JSON list.
pub(super) fn of(host: &dyn Host, job: &ProviderJob) -> Warned {
    let multi_view = job.tripo.as_ref().is_some_and(|t| t.multi_view)
        || job.meshy.as_ref().is_some_and(|m| m.multi_view);
    let item = WarningsItem {
        asset: job.asset.clone(),
        images: job.images.clone(),
        multi_view,
    };
    match warnings(host.session(), &job.project, &[item]) {
        Ok(mut lists) => {
            let (model, refs): (Vec<_>, Vec<_>) = lists
                .pop()
                .unwrap_or_default()
                .into_iter()
                .partition(|w| w.kind == WarningKind::HasModel);
            Warned {
                refs: to_value(refs),
                model: model.first().map_or(
                    Value::Null,
                    |w| json!({ "title": w.title, "detail": w.detail }),
                ),
            }
        }
        Err(err) => {
            tracing::warn!(job = %job.id, error = %err.message, "could not check the refs");
            Warned {
                refs: json!([]),
                model: Value::Null,
            }
        }
    }
}
