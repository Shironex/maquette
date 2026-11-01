//! What the MCP server tells an agent when it connects: where to start, the
//! tool groups, what waits for a person and the skills. What a single tool
//! does and refuses belongs in that tool's description, not here.

pub(super) const INSTRUCTIONS: &str = "\
Maquette turns concept art into real-time 3D web scenes and game models, on this computer.

START HERE
1. get_started reads the workspace, tools, saved keys, projects and gates, and lists what to do next.
2. create_project makes a website or game project when there is none to work in. It spends nothing.
3. Follow the next list in an answer. An error carries code, and fix and tryTool when they help.
Every tool's description starts with its group in brackets, and tools that work on a project take its id as project.

TOOL GROUPS
[start] get_started, create_project.
[project] Projects, assets, the scene layout, copying assets, the asset trash.
[lab] Concept images, their jobs, the style board, picks to the Board, the image backends, Codex usage.
[refs] An asset's reference views: reading and tagging them, drawing missing ones, cleaning backgrounds.
[send] Sending refs to Tripo or Meshy or by hand, redos, splits, credits, By hand handoffs and downloads.
[inbox] Models waiting to be accepted: checks, accept, discard, restore.
[prepare] Real size, fix-ups, parts, materials, assembling props, comparing versions.
[built] Models a Blender script in the project makes, with no credits and no approval.
[pipeline] Running a project, presets, bake settings, starter scripts, how a website ships, the site's dev server.
[game] A game project's repo, format, budgets, imports and ship plan.
[library] Models kept outside any project. It copies, never links, and spends nothing.
[animation] The clip library and baking clips onto a rigged model. Mixamo clips stay on this computer.
[export] Export packs for other engines and render styles: headless Blender, no credits.
[avatar-rig] VRM avatars and the rig doctor. Settings > Agents can turn this group off.

WAITS FOR A PERSON (in the Maquette app, no tool can answer; poll, and never start the work again)
- Spending: send_to_3d, send_batch_to_3d, redo_inbox_model, redo_inbox_models and split_with_provider on Tripo or Meshy (follow with get_provider_job).
- Paid images: a round an agent asks for on openai-images (follow with get_concepts).
- Shipping: run_pipeline at its Ship step while Ask before shipping is on, and always in a game project (follow with get_run). buildOnly stops before Ship.
- Avatars: new weights, rest pose and spring chains (follow with get_avatar). Rig doctor: a joint move, and adding its draft as a version.
- Only a person can: save API keys, set the monthly OpenAI image cap, sign in to Codex, install Blender and the other tools, open a workspace.
A waiting answer gives status \"waiting for approval\", pollEverySeconds and whereToApprove.
Nothing is lost without them: send_to_3d with provider by-hand and add_built_asset spend nothing.

ALWAYS
start_page_preview runs the site repo's dev command on this computer. Blender only runs headless. No tool opens its window or captures the screen. Nothing is deployed.

SKILLS (in the Maquette plugin, when your client has it)
maquette-lab-round: concepts to clean refs.
maquette-by-hand-round: models made on a provider's site, without API credits.
maquette-character-rig: rigging and animating a character.
maquette-avatar-vrm: a character as a VRM avatar.
maquette-game-export: models into a game repo.";

#[cfg(test)]
mod tests {
    use super::INSTRUCTIONS;

    #[test]
    fn the_instructions_stay_short_and_name_only_real_tools() {
        assert!(INSTRUCTIONS.len() < 4000, "{} bytes", INSTRUCTIONS.len());
        let tools = crate::mcp::tools::McpTools::tool_names();
        let verbs = [
            "get_", "set_", "send_", "redo_", "run_", "create_", "add_", "split_",
        ];
        for word in INSTRUCTIONS.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if word.contains('_') && verbs.iter().any(|v| word.starts_with(v)) {
                assert!(tools.iter().any(|t| t == word), "{word} is not a tool");
            }
        }
    }
}
