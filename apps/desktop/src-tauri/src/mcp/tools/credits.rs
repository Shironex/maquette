//! The credits tool: what is left on each provider and what sends spent.

use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{McpTools, Out, reason, respond, to_value};

/// The most spends one answer lists.
const MAX_SPENDS: usize = 200;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreditsArgs {
    /// Only this project's spends. The balances are per account either way.
    pub project: Option<String>,
    /// How many recent spends to list, 20 by default and 200 at most.
    pub limit: Option<u32>,
    /// Ask Tripo and Meshy for the balance (a free call), true by default.
    pub balance: Option<bool>,
}

#[tool_router(router = credits_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Get the credits: each provider's balance (asked live, a free call; skipped without a saved key), totals for the last 7 and 30 days with the share spent on redos, and the most recent paid sends from the local ledger with provider, task id, settings, credits charged (\"estimated\" when the provider has not reported yet), outcome and whether it was a redo. Reads only; nothing is spent.",
        annotations(read_only_hint = true)
    )]
    async fn get_credits(&self, Parameters(args): Parameters<CreditsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let studio = host.studio();
                let mut spends = studio.ledger().map_err(reason)?.spends;
                if let Some(project) = args.project.as_deref() {
                    spends.retain(|s| s.project == project);
                }
                let totals = ledger::summarise(&spends, now_ms());
                let balances: Value = if args.balance.unwrap_or(true) {
                    to_value(studio.balances())
                } else {
                    Value::Null
                };
                let limit = args.limit.map_or(20, |n| n as usize).min(MAX_SPENDS);
                let recent: Vec<Value> = spends.iter().take(limit).map(to_value).collect();
                Ok(json!({
                    "balances": balances,
                    "totals": to_value(totals),
                    "spends": recent,
                    "more": spends.len().saturating_sub(limit),
                }))
            })
            .await;
        respond(found)
    }
}
