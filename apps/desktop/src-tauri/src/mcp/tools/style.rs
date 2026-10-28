//! Tools that read and change a project's style board, the look every new
//! Lab direction and object ref carries. They spend nothing.

use maquette_core::concepts::{StyleBoard, StyleBoardChange, style};
use maquette_core::project;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::lab;

use super::{McpTools, Out, ProjectArgs, reason, respond, to_value, workspace};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetStyleArgs {
    /// The project id.
    pub project: String,
    /// One written style line, 200 characters at most, for example
    /// "frosted glass, cool night light, chunky silhouettes". "" clears it.
    pub line: Option<String>,
    /// Up to 6 colours as #rrggbb (or #rgb). [] clears the palette.
    pub palette: Option<Vec<String>>,
    /// The whole new list of images, 4 at most, in order: board images as
    /// get_style_board lists them ("style/..."), concepts and refs of the
    /// project as get_concepts lists them ("concepts/<direction>/<file>.png",
    /// "refs/<file>.png"), or absolute paths inside the workspace. Each is
    /// copied into the project's style/ folder. [] empties the board's images.
    pub images: Option<Vec<String>>,
}

const NOTE: &str = "The board lives in the project's style.json, its images copied into style/. Every new Lab direction and object ref carries it (the line and palette after the brief, the images attached after the brief's own refs) unless started with useStyle false. Each change bumps revision; jobs record the revision they carried, and a retry carries the same.";

fn board_value(project: &str, board: &StyleBoard) -> Value {
    json!({
        "project": project,
        "empty": board.is_empty(),
        "board": to_value(board),
        "note": NOTE,
    })
}

#[tool_router(router = style_router, vis = "pub(super)")]
impl McpTools {
    #[tool(
        description = "Read a project's style board: the written style line, the palette (#rrggbb) and up to 4 reference images (copies in style/, with the concept or ref each came from), and its revision. Every new Lab direction and object ref carries it unless started with useStyle false.",
        annotations(read_only_hint = true)
    )]
    async fn get_style_board(&self, Parameters(args): Parameters<ProjectArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let (root, config) = workspace(host)?;
                project::load(&root, &config.layout, &args.project).map_err(reason)?;
                let dir = project::dir(&root, &config.layout, &args.project).map_err(reason)?;
                let board = style::load(&dir).map_err(reason)?;
                Ok(board_value(&args.project, &board))
            })
            .await;
        respond(found)
    }

    #[tool(
        description = "Change a project's style board: the style line, the palette and the images (concepts or refs of the project by their file, or image paths inside the workspace; each is copied into the project's style/ folder, never linked). Only the fields given change; images replaces the whole list. It spends nothing and the app's Lab updates live."
    )]
    async fn set_style_board(&self, Parameters(args): Parameters<SetStyleArgs>) -> Out {
        let found = self
            .blocking(move |host, _| {
                let change = StyleBoardChange {
                    line: args.line,
                    palette: args.palette,
                    images: args.images,
                };
                let board = lab::set_style(host.session(), &args.project, &change, false)
                    .map_err(|e| e.message)?;
                host.style_changed(&args.project, &board);
                Ok(board_value(&args.project, &board))
            })
            .await;
        respond(found)
    }
}
