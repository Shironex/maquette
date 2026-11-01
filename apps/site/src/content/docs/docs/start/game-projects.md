---
title: Game projects
description: Make and fix models for your own engine, keep each folder within its budget, and ship plain model files into the game repo after you approve the change.
sidebar:
  label: Game projects
---

A game project makes models for your own engine instead of a web scene. It starts from the models the game already has, fixes and exports them in the format the engine reads, and Ship writes them back into the game repo. Ship always waits for you to approve it in the app and never commits.

## Create one

In New project, pick **Game** under Ships to, then the **Game repo**. Maquette reads the repo and says what it found:

| Check            | What it looks for                                                                                                                                 |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Git              | A git repo and the branch it is on. Ship still writes into a folder that is not one.                                                              |
| CMake            | `CMakeLists.txt` at the root and the executables it builds (`add_executable`).                                                                    |
| Model folder     | The first of `assets/models`, `assets/meshes`, `models`, `res/models` and `data/models` that exists. Its models are read up to four folders deep. |
| Format           | `.glb` or `.obj`, whichever most of the files are. The format picker starts on it, marked detected.                                               |
| Loader           | The library the engine reads models with (cgltf, tinygltf, fastgltf, tinyobjloader or Assimp), named in the build files or a vendored folder.     |
| KTX2 and meshopt | Whether the engine can decode them. Game projects never use them either way.                                                                      |

You can change the format and the model folder before you create the project, and the model folder later in Settings. New project also offers the [game templates](#game-templates), which can set the textures too. A game still starts from its own models.

A website repo with a `CMakeLists.txt` gets a hint to pick Game instead. A website project's check line names the folders its `maquette.site.json` gives, never a fixed path.

## Import the game's models

Right after you create the project, and from **Import models** in the Board's Add menu, Maquette scans the model folder and lists every model with its folder, format, triangles, textures, size and its folder's budget check. Pick the ones Maquette should look after. Each becomes an asset whose version 1 is the repo file, and joins the ship record, so a later Ship may replace it. Models you leave out stay hand made: Ship never touches them. An import changes nothing in the game repo.

Each imported model gets a preview render in the background, so the Board cards fill in as Blender finishes. A project imported before previews came with the import shows **Render previews** with its count in the Board header's ... menu, which renders every model that has none.

## Budgets per folder

Every model checks its triangles and its largest texture against the budget of the folder it lives in (`props/`, `enemies/`, the top level), or of the nearest folder above it. There is no scene total. A new game starts with 12k triangles and 1024 px for `props`, 40k and 2048 px for `enemies`, 60k and 2048 px for `characters`, and 20k and 1024 px for any other folder.

The Board marks a model over its budget on its card and in the list, and the top bar counts the models over. The Inspector's **Budget** tab lists every model against its budget, lets you change each folder's numbers, and **Add the fix-ups** gives every model over budget a Decimate or Shrink textures fix-up. Build models applies them to a copy; the repo file stays version 1.

The Budget tab's **Game folder** column moves one model to a folder of its own inside the model folder (`kit`, `enemies/bosses`), keeping its file name. Clear it to go back to the folder its kind or its import gives it. The next Ship writes the model at the new path and removes the old file, but only when Maquette wrote it and nothing else ships there; a hand made file is never removed.

A rigged model (one with bones) never gets Decimate, since it would tear the skin weights and shape keys. Over its triangle budget it reads **By hand**: reduce it in Blender yourself. Shrink textures still applies to it.

## The game pipeline

| Step         | What it does                                                                                                                            |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Build models | Brings in every model and applies its fix-ups, headless.                                                                                |
| Bake light   | Off by default, since games light at runtime. Turn it on in the Game export panel.                                                      |
| Export       | One plain file per model in the engine's format, under `web/raw/` at the path it has in the game (`props/brazier.glb`).                 |
| Game export  | Lays the files out as the model folder keeps them in `web/game/` and checks every one. A model nothing changed is copied byte for byte. |
| Ship         | Writes `web/game/` into the model folder, after you approve it.                                                                         |

Game projects have no Capture posters and no camera stops. The stage after Blender is called Export.

The **Game export** panel in the Pipeline view sets what the engine reads:

- **Format**: `.glb` (binary glTF, textures inside) or `.obj` with its `.mtl` and texture files beside it.
- **Textures**: PNG, JPG, or Keep each image's own format.
- **Texture size cap**: 256 to 4096 px on the longest edge; smaller textures stay as they are. A model in a folder with a budget exports at that budget's texture size instead, so `props/` at 1024 px caps its props at 1024 px. The cap covers the models in folders without a budget, and the panel lists each folder's size.
- KTX2, meshopt and Draco are always off. Game export refuses an exported model that uses them, or a texture over the cap.
- **Drop normal and metal maps**: leaves out the normal and metal-roughness textures, for engines that light flat or pack their own maps. Color and emission stay.

Rigs and animation clips export with a `.glb`. Each rigged model keeps only its own clips, never another rig's. A clip switched off in Motion stays out of the model's game file. LOD levels are shown in the panel but not built yet.

A project made before the per folder cap keeps its own copies of `export_game.py` and `game_export.mjs`. The Pipeline view offers to update them; until then every model exports at the global cap.

## Game templates

A template fills export settings you could also set by hand: the model format, the texture format, the texture size cap, whether normal and metal-roughness maps stay, and a budget row for each folder it lays out. It adds no assets and no game code, never moves or deletes a file, and the project does not remember it, so you keep editing the settings as usual. Pick one in New project, or use **Use a template** in the Game export panel. The picker groups them by shape of the models, by engine and by device or kind of project, and each card shows its format, textures and triangle budgets.

| Template        | Format        | Textures                                      | Folder and budget (triangles, texture px)                             |
| --------------- | ------------- | --------------------------------------------- | --------------------------------------------------------------------- |
| Prop set        | `.glb`        | as the game has them                          | `props/` 12k, 1024                                                    |
| Character       | `.glb`        | as the game has them                          | `characters/` 60k, 2048                                               |
| Level kit       | `.glb`        | as the game has them                          | `kit/` 5k, 1024                                                       |
| Level kit, .obj | `.obj + .mtl` | as the game has them                          | `kit/` 5k, 1024                                                       |
| Web game        | `.glb`        | JPG, cap 1024                                 | `props/` 10k, 1024; `characters/` 25k, 1024; `kit/` 3k, 512           |
| Godot           | `.glb`        | PNG, cap 2048                                 | `props/` 20k, 2048; `characters/` 50k, 2048; `environment/` 30k, 2048 |
| Unity           | `.glb`        | PNG, cap 2048                                 | `props/` 15k, 1024; `characters/` 40k, 2048; `kit/` 5k, 1024          |
| Unreal          | `.glb`        | PNG, cap 4096                                 | `props/` 40k, 2048; `characters/` 80k, 4096; `environment/` 60k, 2048 |
| Mobile          | `.glb`        | JPG, cap 1024                                 | `props/` 4k, 512; `characters/` 15k, 1024; `kit/` 2k, 512             |
| VR standalone   | `.glb`        | JPG, cap 1024                                 | `props/` 5k, 1024; `characters/` 20k, 1024; `kit/` 2k, 512            |
| Game jam        | `.glb`        | JPG, cap 1024, normal and metal maps left out | the model folder's top level 20k, 1024                                |

Every new template writes `.glb`. Game export still writes plain files: no KTX2 or meshopt, whichever template you pick.

**Which numbers are limits and which are judgment.** None of the per model rows is a published limit; they are judgment, picked as sensible starting points, and each template's text in the app says so. The one published figure is Meta's, for the whole view of a standalone headset: its device comparison page lists about 1.5M triangles and 200 draw calls on Quest 3 and 750K and 100 on Quest 2. VR standalone's per model rows are chosen so a view of a few dozen models stays under that, which is a judgment too. The Unity and Unreal templates only say to check that your project reads `.glb`; they make no claim about how either engine imports it.

Before it applies, the sheet shows what would change in this project: the format, each export setting that differs (texture format, size cap, maps), each budget row before and after, how many models would be over budget, and any model that would land somewhere new. What it saves is exactly what it shows.

- Rows for other folders stay. A template that names no texture setting (the first four) leaves the textures and the size cap as they are; Bake light and the model folder always stay. A folder's own budget texture size still wins over the cap for its models.
- No file moves or goes away. A model whose file is in the model folder keeps its path and its format, also when the template picks another format: one Ship wrote, one you imported, or one that was just there under the name Export would use.
- A model with nothing in the game yet takes the new format in the folder it was going to, and the sheet lists its new path. A new budget row moves no model that exists, so a template in the same format changes no path.
- New props go to `props/` when the game has a budget for it, otherwise `kit/` when it has one. A new scene goes to `environment/` (or `environments/`) when the game has a budget for it, otherwise `kit/`, otherwise `props/`. In a game with several, move a piece with its **Game folder** in the Budget tab. A model that already has a path keeps it when a template adds a folder.
- The sheet warns when most of the game's models are in another format than the template's, when rigged models would lose their rig in `.obj`, when maps would be left out, when JPG would drop transparency, and when Game jam's top level row sits beside folders with their own rows.

## Ship to the game

When a run reaches Ship, the Pipeline view opens the Ship diff and the run waits. The diff lists each file:

- **New**: not in the game yet.
- **Replaced**: a file Maquette wrote or imported before, now different. When the file in the game is still the original Maquette imported, the row says it **replaces the imported original**, and the approval card counts them.
- **Unchanged**: the same bytes, left as they are.
- **Removed**: the old file of a model you moved to another folder. Ship removes it only when Maquette wrote it.
- **Hand made, left alone**: not in the ship record, so Ship never touches it, also when an export would land on it.

It also shows where Ship writes (game repo, model folder, branch), how many files the ship record lists after the run, and the checks before Ship. **Approve and ship** writes the files; **Deny** stops the run and nothing in the game repo changes. Ship never removes a file other than a moved model's old one, and never commits: review the change in git and commit it yourself. When git status lists uncommitted files in the game repo, the diff and the approval card suggest committing them first, so the Ship change is easy to review and undo. Maquette only reads git status; it never writes to the repo's git, and for a repo an agent named or set it does not read it either, so the uncommitted count is not read there. **What Ship changes** in the Pipeline view shows the same diff between runs.

## Views

Game projects hide the Page and Timeline views; Motion stays. Every view's top bar shows the model budget (how many models are over their folder's budget) and the stages Concept, Refs, 3D model, Blender, Export and Ship, never a scene MB budget. Motion shows only the clips of each model: a game export keeps a clip in the model file unless its switch says to leave it out of the game file, and there are no states or scene budget, and the list marks the models over their folder's budget. Long clip names truncate with the full name on hover, and a model with eight clips or more gets a search box. The Inspector shows Compare and Budget, without Layout and Paths. Its model panel checks the selected model's triangles against its folder's budget and leaves out the website's compression and scene total. Game preview, the model in the game's own lighting, is listed as planned and is not built yet.

## Agents

Over MCP, `get_game` reads a game project's settings and each model against its budget, `scan_game_models` and `import_game_models` scan and import the model folder (an import asks for each model's preview), `set_game_settings` changes the format, model folder, export (`dropMaps` too), bake, budgets, each asset's own folder (`assetFolders`, null to go back) and the game repo (`repo`, an absolute path to a git checkout or a worktree), or applies a game template (`template`; `get_game` lists the templates under `templates` with ids, names and numbers, and `previews` with ids or `"all"` adds what each would change; before this, every call returned every preview, and the entries still hold the same `template` field first), and `get_ship_plan` shows what Ship would change, which replaced files are imported originals, and how many files the game repo has uncommitted (not read for a repo an agent named or set). `get_ship_plan` and `run_pipeline` also take `repo` for that one plan or run, so an agent can ship into a worktree without changing the project. `run_pipeline` runs the game pipeline; Ship waits for a person to approve it in the app, whoever started the run.
