//! How a project ships. `ship.json` and its checks live in the model crate;
//! the ship plan and the size report stay here.

pub use maquette_model::ship::*;

mod plan;
mod report;
#[cfg(test)]
mod tests;

pub use plan::{
    BUILD_ORIGIN_MARKER, EXPORT_ORIGIN_MARKER, SHIP_SCRIPT_MARKER, ShipTarget, SiteLeftFile,
    SitePlanFile, SiteShipPlan, origin_ready, plan, script_ready,
};
pub use report::{SizeReport, SizeRow, SizeTotals, size_report};
