//! The credits tool: what is left on each provider and what sends spent.

use crate::mcp::tools::ToolError;
use maquette_core::pipeline::history::now_ms;
use maquette_core::providers::ledger::{self, ImageSpend, images::month_start_ms};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{McpTools, Out, respond, to_value, workspace};
use crate::mcp::host::Host;

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
        description = "Get the credits: each provider's balance (asked live, a free call; skipped without a saved key), totals for the last 7 and 30 days with the share spent on redos, and the most recent paid sends from the local ledger with provider, task id, settings, credits charged (\"estimated\" when the provider has not reported yet), outcome and whether it was a redo. openaiImages has the Lab's OpenAI Image API dollars: monthToDateUsd (this local calendar month, every project), capUsd, leftUsd, pendingUsd (jobs queued or waiting for approval), imagesThisMonth, the quality, perImageUsd (what an image has cost lately, the figure the cap check uses) and the most recent image lines (projectMonthUsd too when project is given). Reads only; nothing is spent.",
        annotations(read_only_hint = true)
    )]
    async fn get_credits(&self, Parameters(args): Parameters<CreditsArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let studio = host.studio();
                let mut spends = studio.ledger()?.spends;
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
                let images = openai_images(host, args.project.as_deref(), limit)?;
                Ok(json!({
                    "balances": balances,
                    "totals": to_value(totals),
                    "spends": recent,
                    "more": spends.len().saturating_sub(limit),
                    "openaiImages": images,
                }))
            })
            .await;
        respond(found)
    }
}

fn usd(n: f64) -> f64 {
    (n * 1000.0).round() / 1000.0
}

/// The month's OpenAI image dollars against the cap, from `credits.jsonl`.
fn openai_images(host: &dyn Host, project: Option<&str>, limit: usize) -> Result<Value, ToolError> {
    let (root, config) = workspace(host)?;
    let settings = config.lab.images;
    let lines = ledger::read(&root)?.images;
    let now = now_ms();
    let start = month_start_ms(now);
    let this_month: Vec<&ImageSpend> = lines.iter().filter(|l| l.at_ms >= start).collect();
    let spent = ledger::month_to_date(&lines, now);
    let cap = f64::from(settings.monthly_cap_usd);
    let per_image = ledger::recent_average_usd(&lines, settings.quality).unwrap_or_else(|| {
        maquette_core::providers::openai_images::estimate_usd(settings.quality, 0)
    });
    let mut out = json!({
        "monthToDateUsd": usd(spent),
        "capUsd": settings.monthly_cap_usd,
        "leftUsd": usd((cap - spent).max(0.0)),
        "pendingUsd": usd(host.concepts().api_pending_usd()),
        "imagesThisMonth": this_month.len(),
        "quality": to_value(settings.quality),
        "perImageUsd": usd(per_image),
        "recent": lines
            .iter()
            .filter(|l| project.is_none_or(|p| l.project == p))
            .take(limit)
            .map(to_value)
            .collect::<Vec<_>>(),
    });
    if let Some(project) = project {
        let mine: f64 = this_month
            .iter()
            .filter(|l| l.project == project)
            .map(|l| l.usd.max(0.0))
            .sum();
        out["projectMonthUsd"] = json!(usd(mine));
    }
    Ok(out)
}
