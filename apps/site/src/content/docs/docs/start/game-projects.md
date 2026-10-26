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

You can change the format and the model folder before you create the project, and the model folder later in Settings. There are no game templates: a game starts from its own models.

A website repo with a `CMakeLists.txt` gets a hint to pick Game instead. A website project's check line names the folders its `maquette.site.json` gives, never a fixed path.

## Import the game's models

Right after you create the project, and from **Import models** on the Board, Maquette scans the model folder and lists every model with its folder, format, triangles, textures, size and its folder's budget check. Pick the ones Maquette should look after. Each becomes an asset whose version 1 is the repo file, and joins the ship record, so a later Ship may replace it. Models you leave out stay hand made: Ship never touches them. An import changes nothing in the game repo.

## Budgets per folder

Every model checks its triangles and its largest texture against the budget of the folder it lives in (`props/`, `enemies/`, the top level), or of the nearest folder above it. There is no scene total. A new game starts with 12k triangles and 1024 px for `props`, 40k and 2048 px for `enemies`, 60k and 2048 px for `characters`, and 20k and 1024 px for any other folder.

The Board marks a model over its budget on its card and in the list, and the top bar counts the models over. The Inspector's **Budget** tab lists every model against its budget, lets you change each folder's numbers, and **Add the fix-ups** gives every model over budget a Decimate or Shrink textures fix-up. Build models applies them to a copy; the repo file stays version 1.

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
- **Texture size cap**: 256 to 4096 px on the longest edge; smaller textures stay as they are.
- KTX2, meshopt and Draco are always off. Game export refuses an exported model that uses them, or a texture over the cap.

Rigs and animation clips export with a `.glb`. LOD levels are shown in the panel but not built yet.

## Ship to the game

When a run reaches Ship, the Pipeline view opens the Ship diff and the run waits. The diff lists each file:

- **New**: not in the game yet.
- **Replaced**: a file Maquette wrote or imported before, now different.
- **Unchanged**: the same bytes, left as they are.
- **Hand made, left alone**: not in the ship record, so Ship never touches it, also when an export would land on it.

It also shows where Ship writes (game repo, model folder, branch), how many files the ship record lists after the run, and the checks before Ship. **Approve and ship** writes the files; **Deny** stops the run and nothing in the game repo changes. Ship never removes a file and never commits: review the change in git and commit it yourself. **What Ship changes** in the Pipeline view shows the same diff between runs.

## Views

Game projects hide the Page and Timeline views; Motion stays. The Inspector shows Compare and Budget, without Layout and Paths. Game preview, the model in the game's own lighting, is listed as planned and is not built yet.

## Agents

Over MCP, `get_game` reads a game project's settings and each model against its budget, `scan_game_models` and `import_game_models` scan and import the model folder, `set_game_settings` changes the format, model folder, export, bake and budgets, and `get_ship_plan` shows what Ship would change. `run_pipeline` runs the game pipeline; Ship waits for a person to approve it in the app, whoever started the run.
