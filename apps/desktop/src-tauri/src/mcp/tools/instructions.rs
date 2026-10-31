//! What the MCP server tells an agent when it connects: where to start,
//! what each group of tools does and which of them wait for a person.

pub(super) const INSTRUCTIONS: &str = "Maquette turns concept art into real-time 3D web scenes. \
Start with list_projects, then get_pipeline or list_assets for a project. \
run_pipeline runs a project's pipeline; when it reaches the Ship step it may answer \
\"waiting for approval\": a person approves or denies shipping in the Maquette app, \
so poll get_run instead of starting the run again. send_to_3d sends an asset's refs to \
Tripo or Meshy (it always waits for a person to approve the credits in the app; follow it \
with get_provider_job) or writes a by hand handoff folder (on Tripo with mesh, HD Model or \
Smart Mesh (P2.0), and rig, biped, quadruped, multi-legged or none, for its settings card); \
send_batch_to_3d sends several assets at once under one approval. redo_inbox_model sends an inbox model's asset again \
prefilled from its send (redo_inbox_models for several), under the same approval. get_credits \
reads the provider balances and the local ledger of paid sends (API spend and By hand \
subscription spend apart). list_handoffs lists the By hand sends waiting for a model made on \
the provider's site, the export file name each waits for and what the Downloads watcher \
matched; it reads only. A download whose name is only close to a send's waits there under \
needsLook, and one it could not place under unmatched: nothing lands without attach_download \
(into the inbox for the asset you name, ready for accept_inbox_model) or dismiss_download. \
add_model_file takes a model file someone fixed by hand (an absolute .glb, .gltf, .fbx or \
.obj path outside the workspace's versions) into the inbox as an asset's next version the same \
way, and accepts it at once with accept true. accept_inbox_models accepts several inbox models in \
one call, each answering on its own; restore_inbox_model puts a discarded one back (list_inbox \
lists them under discarded). \
cancel_provider_jobs stops sends by id (a by hand send stops waiting, a send not yet started is cancelled, one that cannot be stopped says why), but a send waiting for approval is only denied by a person in the app. make_object_refs turns the objects of one concept into \
one isolated image each. get_ref_views reads an asset's refs and the view each shows \
(front, left, back, right or other); set_ref_views sets them, and every send then carries \
front, left, back and right in that order and leaves other refs home. make_other_views draws \
an asset's missing views from its front and clean_ref_background makes a cleaned \
copy of one ref; both make images on the Lab's image backend like start_concepts, and make_other_views takes notes \
for every view's prompt. use_ref_candidate puts a candidate in place: a clean copy's original \
moves to refs/originals, and a drawn view replaces the ref in its slot, which moves to \
refs/originals too unless another asset or a send still uses it (replace false keeps it as \
other); its answer says what moved where. send_to_3d and \
send_batch_to_3d report refWarnings (small, cropped, busy background, one view only, a view that looks different from the front) before \
anything is spent; they never stop a send. get_concepts reads the Lab, with its failed jobs; a job Codex rate \
limits runs again on its own a few times, a job that meets the Codex usage limit waits with \
the rest of the queue until it lifts (usageLimit, retryAtMs), and retry_concept_jobs runs failed ones again (same \
prompt, refs and file, no new direction; include held or all runs the rate or usage limited ones now too). Jobs \
the app closed on come back failed with an Interrupted by a restart message, ready for retry_concept_jobs. \
cancel_concept_jobs cancels queued, running, waiting or approval held jobs as the Lab's Cancel does, and \
prioritize_concept_jobs moves a project's queued jobs to the front of the line (Run next). get_codex_usage reads how much of Codex's five hour and weekly \
windows is used and when they reset; retry_usage_limited_jobs runs the jobs the usage limit holds once a \
fresh read says it has lifted. Lab images are made by Codex unless Settings or a request's \
backend picks openai-images (OpenAI's Image API, paid per image within a monthly cap in US dollars); \
get_image_backend and set_image_backend read and change that (the backends and quality; the cap is \
set only by a person in the app), and every job reports its backend. A round \
an agent asks for on openai-images (start_concepts, make_object_refs, make_other_views, \
clean_ref_background, retry_concept_jobs) waits for a person to approve it in the app and answers \
\"waiting for approval\" with its estimate: poll get_concepts and never start it again. \
get_credits shows the month's OpenAI image dollars against the cap. pick_concepts puts concepts on the Board \
as Send picks to Refs does: each image goes into refs/ and becomes or joins an asset at Refs \
(named after its object for object refs, else after its direction), ready for send_to_3d; \
picking an image again returns its asset. star_concepts stars or unstars them. Both spend \
nothing and the app updates live. \
get_style_board reads a project's style board (a style line, a palette and up to 4 images \
copied into style/); set_style_board changes it from concepts, refs or image paths inside the \
workspace. Every new start_concepts direction and make_object_refs request carries the board \
after its own prompt and refs unless useStyle is false; both spend nothing. \
set_compression_presets changes the per model texture size, data map size, codec and \
meshopt level that get_pipeline shows (it spends nothing; locked rows refuse). \
get_material_tweaks reads an asset's material names, tweaks and per map codecs; \
set_material_tweaks changes its tint, roughness, metalness and emissive strength (for every \
material or one by name) and the ETC1S or UASTC codec of its color, normal and other maps; \
Compress applies them to the shipped model on the next run. \
get_avatar reads a character asset's VRM avatar (readiness with the bone map and findings, the meta, \
the last reweight, the last normalised rest pose with its before and after sheets, the parts that \
could swing and the last spring chains, the last export); set_avatar changes its meta, bone \
overrides, material mode (mtoon-soft or pbr) and confirmed springs (by candidate id, or \"none\"); \
build_avatar writes the project's \
exports/<asset>.vrm, never a new version. When cloth or hair weighted to the arms needs freeing \
(reweight auto, the default) and when the rest pose needs normalising (headless, elbows and fingers \
straighten by default) it answers \"waiting for approval\" with the step and the sheets: a person approves the new \
weights, the new pose and the new spring chains (springs auto, the default, for confirmed springs) \
in the app's Avatar view and no tool can, so call get_avatar or build_avatar again later. \
get_real_size reads an asset's real size and fix-ups with what its model measures; set_real_size \
sets the size its model is scaled to (height, width, depth or longest, cm or m); add_fixups and \
remove_fixups change its fix-ups (decimate, shrink textures, merge, split into parts, apply \
transforms, remove loose parts, origin to bottom centre), which the Build scene step applies headless on a copy of every \
version; list_fixups lists them and preview_fixups shows their numbers before and after. \
get_parts lists an asset's parts by the names they ship as (the export node names object states \
and explode tracks move), set_parts renames, joins and hides them, and compare_versions compares \
two versions (or one before and after its fix-ups) part by part. \
split_with_provider splits a model into parts on Tripo or Meshy (it spends credits, so a person \
approves it in the app, and the split lands in the inbox as the next version; a Meshy split \
bakes its texture back once accepted), and assemble_prop joins part assets into one prop, a \
node per part under one root, as the next version of a target asset. \
set_pad_edges sets whether an asset's texture edges around its UV islands are padded before the \
textures shrink (models accepted from the Inbox pad by default). \
get_bake_settings reads the Bake light step's lightmap size, samples and denoiser and the \
assets that skip the bake; set_bake_settings changes them and set_skip_bake sets assets to skip \
the bake (they keep their own material lighting and ship no lightmap; models accepted from the \
Inbox skip it by default). get_starters compares a project's starter scripts in tools/ \
with the ones this Maquette ships, and update_starters brings the outdated ones up to date, as \
the Pipeline view's Update banner does, for the project named in the call. \
get_layout reads where each model stands in the scene, the camera stops and the first view \
budget; set_layout changes them (metres, y up, degrees) and the app updates live; the next \
run_pipeline rebuilds the scene and its posters from it. set_layout also sets a prop's \
object states (poses of its parts, transitions and triggers), whether a model's clips ship \
the scroll timeline's explode tracks (how far a model's parts spread apart at each \
camera stop), and paths: curves through the scene the site draws as tubes, with energize \
tracks saying how much of each is lit at each camera stop (Ship sends them all to the site \
with the rest of the layout). \
get_motion reads every model's rig, clips and parts and what the clips weigh inside the scene \
budget. copy_asset copies an asset with its refs and \
versions to another project. remove_assets removes assets from a project into the workspace \
trash (refused while its model is being made or a run is going), list_removed_assets lists \
them and restore_asset puts one back. \
The library keeps models outside any project (library/ at the workspace root). list_library \
reads it, 25 compact entries per page (follow nextCursor, or pass full for everything about an entry; \
collection narrows it to one collection); add_to_library copies an asset's newest model in (a newer model of an asset already \
there becomes its entry's next version), and add_many_to_library adds several (dryRun first says what each \
would become); use_from_library copies an entry into a project as a \
new asset that remembers its origin; update_from_library brings a newer library version in as \
the asset's next version, keeping the fix-ups that still apply. tag_library_entries, rename_library_entry and \
set_library_collection tidy entries; remove_from_library moves entries to library/.removed and \
restore_to_library brings them back; find_look_alikes reads entries that may be one model twice and \
merge_library_entries merges one into another (restore_to_library on the merged entry undoes it). The library copies, never \
links: nothing changes a project until someone updates it, and none of it spends credits. \
export_pack exports library entries, a collection or a project's models as one pack (OBJ or GLB, \
textures, README, manifest and contact sheet, zipped) inside the workspace: Blender runs headless on \
each model and opens it again to check it, Mixamo clips are always left out, and dryRun plans it first; \
get_pack and list_packs read packs and presets, and decide_pack raises a budget, keeps a warning, zips \
or stops and resumes. \
The animation library keeps clips once, skeleton only (library/animations). list_animations \
reads it; import_animations reads Mixamo FBX files or pack zips inside the workspace headless \
(the skin is dropped, the rig read from the bone names, duplicates skipped, fixed clips become \
a new revision); save_model_clips saves a rigged model's clips, or a Tripo job's animations in \
the Inbox, as a pack. use_animations bakes library clips onto a rigged model headless (bone \
map, per model clip settings) into its next version after a review that leaves failed clips \
out; get_retarget_review reads a model's last review. Mixamo clips may be used in your \
projects but never redistributed as raw files, so the library stays on this machine and none \
of it spends credits. \
add_built_asset adds a built asset: a model a Blender script in the project makes. Its script \
and helpers are copied into the project's built/ folder and it builds headless into the asset's \
next version, with no credits and no approval; rebuild_built_asset builds one again (or all \
whose files changed) and list_built_assets reads their state and last build. A full \
run_pipeline rebuilds the changed ones first. \
A project ships to a website or to a game (list_projects and get_project give its target). \
For a game project, get_game reads the game repo, model format, model folder, Game export \
settings, Bake light and the budget per folder with each asset's numbers against it; \
set_game_settings changes them; scan_game_models lists the models in the game's model folder \
and import_game_models imports picked ones, each as an asset with the repo file as v1. \
get_ship_plan shows which files Ship would add or replace in the game; Ship never touches \
hand made files and always waits for a person to approve it in the app. \
For a website, get_ship_settings reads how it ships: the style (scene, or models for a \
hand-built site that loads one file per model at its own origin under a stable name), the \
texture format (ktx2, webp or avif), each asset's choice (auto, never, always) and file name, \
the site's models folder and a dry run plan of the next Ship; set_ship_settings changes them \
(the folders go into the site's maquette.site.json), and get_ship_plan returns the same plan. \
get_size_report compares each model as it came in with what ships, in bytes and triangles, \
with CSV and Markdown copies. run_pipeline with buildOnly stops before Ship. \
start_page_preview starts a project's site dev server (its dev command in the site repo), the \
one the app's Page view shows, and waits for it to answer; get_page_preview reads its state, \
address, command and log, and stop_page_preview stops it. Nothing is deployed. \
Blender always runs headless here; there is no tool that opens Blender's window or \
captures the screen.";
