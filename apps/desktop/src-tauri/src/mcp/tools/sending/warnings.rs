//! The weak ref warnings a send reports, the same checks the Send to 3D
//! panel shows. They never stop a send.

use maquette_core::providers::ProviderJob;
use serde_json::{Value, json};

use super::super::to_value;
use crate::commands::refs::{WarningsItem, warnings};
use crate::mcp::host::Host;

/// The warnings for the images a job went with, as a JSON list.
pub(super) fn of(host: &dyn Host, job: &ProviderJob) -> Value {
    let multi_view = job.tripo.as_ref().is_some_and(|t| t.multi_view)
        || job.meshy.as_ref().is_some_and(|m| m.multi_view);
    let item = WarningsItem {
        asset: job.asset.clone(),
        images: job.images.clone(),
        multi_view,
    };
    match warnings(host.session(), &job.project, &[item]) {
        Ok(mut lists) => to_value(lists.pop().unwrap_or_default()),
        Err(err) => {
            tracing::warn!(job = %job.id, error = %err.message, "could not check the refs");
            json!([])
        }
    }
}
