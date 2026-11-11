//! What the MCP server tells an agent when it connects: where to start, the
//! tool groups, what waits for a person and the skills. What a single tool
//! does and refuses belongs in that tool's description, not here.

pub(super) const INSTRUCTIONS: &str = "\
Maquette turns concept art into real-time 3D web scenes and game models, on this computer.

START HERE
1. get_started reads the workspace, tools, saved keys, projects and gates, and lists what to do next.
2. create_project makes a website or game project when there is none. It spends nothing.
3. Follow an answer's next list. Errors carry code, and fix and tryTool when they help.
Each tool's description starts with its [group]; project tools take the project id as project.

TOOL GROUPS (Settings > Agents may turn off any but start and project; toolGroupsOff in get_started names them; ask the person.)
[start] get_started, create_project.
[project] Projects, assets, the scene layout, copying assets, the asset trash.
[lab] Concept images and jobs, the style board, picks to the Board, image backends, Codex usage.
[refs] An asset's reference views: reading and tagging them, drawing missing ones, cleaning backgrounds.
[plan] Planning canvases: presets, nodes, the change log, a website's camera plan. Agents never remove a person's nodes; they ask with propose_removal.
[send] Refs to Tripo, Meshy or by hand, redos, splits, credits, By hand handoffs and downloads.
[inbox] Models waiting to be accepted: checks, accept, discard, restore.
[prepare] Real size, fix-ups, parts, materials, assembling props, comparing versions.
[built] Models a project's Blender script makes, no credits, no approval.
[pipeline] Runs, presets, bake settings, starter scripts, how a website ships, the site's dev server.
[game] A game project's repo, format, budgets, imports and ship plan.
[library] Models outside any project, copied, never linked; free.
[animation] Clips, recipes, grips and baking them onto a rigged model. Mixamo clips stay on this computer. Recipes: use_recipe, update_recipe, set_recipe_keys, get_recipe_pose, apply_grip, bake_recipe.
[export] Packs for other engines and render styles: headless Blender, no credits.
[avatar-rig] VRM avatars and the rig doctor.

WAITS FOR A PERSON (in the Maquette app; poll, never start the work again)
- Spending: send_to_3d, send_batch_to_3d, redo_inbox_model, redo_inbox_models and split_with_provider on Tripo or Meshy (follow get_provider_job).
- Paid images: a round an agent asks for on openai-images (follow get_concepts).
- Shipping: run_pipeline at Ship while Ask before shipping is on; always in a game, with repo, or into a site repo an agent set (follow get_run). buildOnly stops before Ship.
- Page preview: start_page_preview on a repo an agent set, first and when its path or command changes (follow get_page_preview).
- Avatars: new weights, rest pose and spring chains (follow get_avatar). Rig doctor: a joint move, adding its draft as a version.
- Recipes: add_recipe_bake, propose_recipe_save, propose_grip_save (follow get_recipe_bake, get_recipe, get_grip).
- Camera plan: agents propose with propose_camera_plan; the person's Apply writes layout.json.
- Canvases: removal proposals (Keep it or Remove), Mark as fine on a mechanics overlap and the Auto layout after agents switch.
- Only a person can: save API keys, set the OpenAI image cap, sign in to Codex, install Blender and the tools, open a workspace.
A waiting answer gives status \"waiting for approval\", pollEverySeconds and whereToApprove.
Free: send_to_3d with provider by-hand, and add_built_asset.

ALWAYS
start_page_preview runs the site repo's dev command on this computer; only a repo a person set runs it unasked. Blender only runs headless. No tool opens its window or captures the screen. Nothing is deployed.

SKILLS (in the Maquette plugin, when your client has it)
maquette-lab-round: concepts to clean refs.
maquette-by-hand-round: models made on a provider's site, no API credits.
maquette-character-rig: rigging and animating a character.
maquette-animation-recipe: animation recipes.
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
