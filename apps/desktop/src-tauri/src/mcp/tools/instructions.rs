//! What the MCP server tells an agent when it connects: where to start, the
//! tool groups, what waits for a person and the skills. What a single tool
//! does and refuses belongs in that tool's description, not here.

pub(super) const INSTRUCTIONS: &str = "\
Maquette turns concept art into real-time 3D web scenes and game models, on this computer.

START HERE
1. get_started reads the workspace, tools, saved keys, projects, gates and waits, lists what to do next.
2. create_project makes a website or game project when there is none. It spends nothing.
3. Follow an answer's next list. Errors carry code, and fix and tryTool when they help.
Each tool's description starts with its [group]; project tools take the project id as project.

TOOL GROUPS (Settings > Agents may turn off any but start and project; toolGroupsOff in get_started names them; ask the person.)
[start] get_started, create_project.
[project] Projects, assets, layout, copying, thumbnails, asset trash.
[lab] Concept images and jobs, the style board, picks to the Board, image backends, Codex usage.
[refs] An asset's reference views: read, tag, draw missing ones, clean backgrounds.
[plan] Planning canvases: presets, nodes, the change log, a website's camera plan. Agents never remove a person's nodes; they ask with propose_removal.
[send] Refs to Tripo, Meshy or by hand, redos, splits, credits, By hand handoffs and downloads.
[inbox] Models waiting to be accepted: checks, accept, discard, restore.
[prepare] Real size, fix-ups, parts, materials, assembling props, comparing versions.
[built] Blender-script models, free, no approval; replace_with_built_asset swaps one in.
[pipeline] Runs (run_pipeline is the game export), presets, bake settings, starter scripts, ship plan, dev server.
[game] A game project's repo, format, budgets and imports.
[library] Models outside any project, copied, never linked; free.
[animation] Clips, recipes, grips and baking them onto a rigged model. Mixamo clips stay on this computer. Recipes: use_recipe, update_recipe, set_recipe_keys, get_recipe_pose, apply_grip, bake_recipe.
[export] Packs for other engines and render styles: headless Blender, no credits.
[avatar-rig] VRM avatars and the rig doctor.

WAITS FOR A PERSON (in the Maquette app; never resend while waiting, only once failed or cancelled)
- Spending: send_to_3d, send_batch_to_3d, redo_inbox_model(s), retry_provider_jobs and split_with_provider on Tripo or Meshy; by-hand is free but waits for a model (follow get_provider_job).
- Paid images: a round an agent asks for on openai-images (follow get_concepts).
- Shipping: run_pipeline at Ship while Ask before shipping is on; always in a game, with repo, or into a site repo an agent set (follow get_run). buildOnly stops before Ship.
- Page preview: start_page_preview on a repo an agent set, first and when its path or command changes (follow get_page_preview).
- Avatars: new weights, rest pose and spring chains (follow get_avatar). Rig doctor: a joint move, adding its draft as a version (get_rig_doctor).
- Recipes: add_recipe_bake, propose_recipe_save, propose_grip_save (follow get_recipe_bake, get_recipe, get_grip).
- Camera plan: agents propose with propose_camera_plan; the person's Apply writes layout.json.
- Canvases: removal proposals (Keep it or Remove), Mark as fine on a mechanics overlap and the Auto layout after agents switch.
- Only a person can: save API keys, set the OpenAI image cap, sign in to Codex, install Blender and the tools, open a workspace.
Waits answer \"waiting for approval\" (by hand \"waiting for model\") with pollEverySeconds; at stopPolling tell the person and stop.

ALWAYS
start_page_preview runs the site repo's dev command here; only a repo a person set runs it unasked. Blender only runs headless. No tool opens its window or captures the screen. Nothing is deployed.

SKILLS (Maquette plugin, if your client has it)
maquette-lab-round: concepts to clean refs.
maquette-by-hand-round: models from a provider's site, no API credits.
maquette-character-rig: rig and animate a character.
maquette-animation-recipe: animation recipes.
maquette-avatar-vrm: a character as a VRM avatar.
maquette-game-export: models into a game repo.
maquette-built-rebuild: a model remade as a built asset.
maquette-plan-canvas: planning canvases.";

#[cfg(test)]
mod tests {
    use super::INSTRUCTIONS;

    #[test]
    fn the_instructions_stay_short_and_name_only_real_tools() {
        assert!(INSTRUCTIONS.len() < 4000, "{} bytes", INSTRUCTIONS.len());
        let tools = crate::mcp::tools::McpTools::tool_names();
        // Every verb a tool name starts with, so a tool renamed to any of
        // them, or to a new one the text still names, fails here.
        let verbs: std::collections::BTreeSet<&str> =
            tools.iter().filter_map(|t| t.split('_').next()).collect();
        for word in INSTRUCTIONS.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if word.contains('_') && word.split('_').next().is_some_and(|v| verbs.contains(v)) {
                assert!(tools.iter().any(|t| t == word), "{word} is not a tool");
            }
        }
    }

    #[test]
    fn the_instructions_name_only_skills_of_the_plugin() {
        let skills = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../plugins/maquette/skills");
        let (_, section) = INSTRUCTIONS
            .split_once("\nSKILLS")
            .expect("a skills section");
        let named: Vec<&str> = section
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(':').map(|(name, _)| name))
            .collect();
        let mut on_disk: Vec<String> = std::fs::read_dir(&skills)
            .expect("the plugin's skills folder")
            .filter_map(Result::ok)
            .filter(|e| e.path().join("SKILL.md").is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        on_disk.sort();
        let mut listed: Vec<String> = named.iter().map(|n| n.to_string()).collect();
        listed.sort();
        assert_eq!(
            listed, on_disk,
            "the skills list and the plugin's skills differ"
        );
        for name in named {
            assert!(
                skills.join(name).join("SKILL.md").is_file(),
                "{name} is not a skill in plugins/maquette/skills"
            );
        }
    }
}
