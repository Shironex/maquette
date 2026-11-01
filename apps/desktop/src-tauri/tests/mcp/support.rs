//! What every MCP integration test shares: the client type, the tool call
//! helpers and the plain test host most tests serve the tools through.

use maquette_core::concepts::Concepts;
use maquette_core::pipeline::runner::PipelineRunner;
use maquette_core::pipeline::tools::Toolbox;
use maquette_core::providers::ProviderJobs;
use maquette_core::thumbs::Thumbnails;
use maquette_desktop_lib::mcp::host::Host;
use maquette_desktop_lib::session::Session;
use rmcp::model::{CallToolRequestParams, CallToolResult, ClientConfig, ContentBlock};
use rmcp::service::{RoleClient, RunningService};
use serde_json::Value;

/// A connected MCP client.
pub(crate) type Client = RunningService<RoleClient, ClientConfig>;

/// A host over the app's services with the default toolbox, for tests that
/// do not count the change events a tool sends.
pub(crate) struct TestHost {
    pub(crate) session: Session,
    pub(crate) runner: PipelineRunner,
    pub(crate) concepts: Concepts,
    pub(crate) thumbs: Thumbnails,
    pub(crate) studio: ProviderJobs,
}

impl Host for TestHost {
    fn session(&self) -> &Session {
        &self.session
    }
    fn runner(&self) -> &PipelineRunner {
        &self.runner
    }
    fn concepts(&self) -> &Concepts {
        &self.concepts
    }
    fn thumbs(&self) -> &Thumbnails {
        &self.thumbs
    }
    fn studio(&self) -> &ProviderJobs {
        &self.studio
    }
    fn toolbox(&self) -> Toolbox {
        Toolbox::default()
    }
}

/// The first text block of a tool result.
pub(crate) fn text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Calls `tool` with `args` and returns the result as it came.
pub(crate) async fn raw_call(client: &Client, tool: &str, args: Value) -> CallToolResult {
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Value::Object(map) = args {
        params = params.with_arguments(map);
    }
    client.call_tool(params).await.expect("call")
}

/// Calls `tool`, which must succeed, and parses its JSON answer.
pub(crate) async fn call(client: &Client, tool: &str, args: Value) -> Value {
    let result = raw_call(client, tool, args).await;
    assert_ne!(
        result.is_error,
        Some(true),
        "{tool} failed: {}",
        text(&result)
    );
    serde_json::from_str(&text(&result)).expect("json")
}

/// Calls `tool`, which must fail, and returns the message in its error.
pub(crate) async fn call_error(client: &Client, tool: &str, args: Value) -> String {
    let result = raw_call(client, tool, args).await;
    assert_eq!(result.is_error, Some(true), "{tool} should fail");
    let body = text(&result);
    serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v["error"].as_str().map(str::to_string))
        .unwrap_or(body)
}
