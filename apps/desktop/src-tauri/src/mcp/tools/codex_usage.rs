//! Codex usage for agents: how much of the five hour and weekly windows is
//! used, and running the jobs the usage limit holds once it has lifted.

use maquette_core::concepts::codex_usage::{CodexUsage, UsageWindow};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use crate::commands::codex_usage;

use super::concepts::job_value;
use super::{McpTools, Out, respond};

fn window_value(w: Option<&UsageWindow>) -> Value {
    w.map_or(Value::Null, |w| {
        json!({
            "usedPercent": w.used_percent,
            "windowMins": w.window_mins,
            "resetsAtMs": w.resets_at_ms,
        })
    })
}

pub(super) fn usage_value(usage: &CodexUsage) -> Value {
    json!({
        "known": usage.known,
        "reason": usage.reason,
        "readAtMs": usage.read_at_ms,
        "fiveHour": window_value(usage.primary.as_ref()),
        "week": window_value(usage.secondary.as_ref()),
        "plan": usage.plan,
        "limitReached": usage.limit_reached,
        "limitActive": usage.limit_active(),
        "resetsAtMs": usage.resets_at_ms(),
        "ordinaryUsageAllowed": usage.ordinary_usage_allowed,
        "canRunNow": usage.can_run_now(),
    })
}

#[tool_router(router = codex_usage_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Read how much Codex usage is left, the same read as the status bar's Codex item: fiveHour and week with usedPercent and resetsAtMs (ms since the epoch), limitActive and resetsAtMs while the usage limit holds, and canRunNow. known is false with a reason when this Codex CLI cannot say (too old, not signed in, no answer within 10 s). Lab jobs waiting for the usage limit move to the reset time it names. Spends nothing.",
        annotations(read_only_hint = true)
    )]
    async fn get_codex_usage(&self) -> Out {
        let found = self
            .blocking(move |host, _| {
                let tools = host.toolbox();
                let usage = codex_usage::read_now(&tools, host.concepts());
                host.codex_usage_seen(&usage);
                Ok(usage_value(&usage))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Run the Lab jobs waiting for the Codex usage limit now, as Limit lifted, retry now in the Lab does: the usage is read again, and only when Codex allows ordinary usage and the five hour window is not full does every held job (in every project) run at once with its own prompt, refs and file. Otherwise nothing runs and the answer says until when the limit holds; the jobs still run on their own then. Each run spends Codex usage. Returns lifted, the jobs and the usage.",
        annotations(idempotent_hint = false)
    )]
    async fn retry_usage_limited_jobs(&self) -> Out {
        let found = self
            .blocking(move |host, _| {
                let tools = host.toolbox();
                let result = codex_usage::lift(&tools, host.concepts());
                host.codex_usage_seen(&result.usage);
                let jobs: Vec<Value> = result.jobs.iter().map(job_value).collect();
                Ok(json!({
                    "lifted": result.lifted,
                    "message": result.message,
                    "jobs": jobs,
                    "usage": usage_value(&result.usage),
                }))
            })
            .await;
        respond(found)
    }
}
