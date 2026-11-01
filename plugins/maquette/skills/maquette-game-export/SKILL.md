---
name: maquette-game-export
description: Set up, build and ship models of a Maquette GAME project into a game repo (or a worktree of it): import the game's own models, set export format, textures and per folder budgets, plan and run the pipeline, wait for the person to approve Ship, and check the written files. Use when asked to export or ship models to a game, fix models over budget, change a model's folder or real size, or bake clips into game models. Not for website projects.
---

# Maquette game export

A game project makes and fixes models for the person's own engine. It starts from the models the
game already has, applies fix-ups headless, exports plain files in the engine's format, and Ship
writes them into the game repo after the person approves. Ship never commits. Tools are Maquette
MCP tools (`/docs/agents/mcp-tools/`); `project` is the id `list_projects` returns.

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list. No project to work in: `create_project` makes one and spends nothing. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.

## Hard rules

- Ship ALWAYS waits for a person to approve it in the app, whoever started the run and whatever
  settings say. `run_pipeline` then answers `waiting for approval`. Poll `get_run`; NEVER start the
  run again, and never call `cancel_run` unless the person asks you to stop it.
- Never commit, push or stash in the game repo. Report the changed files and let the person review
  them in git. Maquette only reads git status.
- Never edit a game file by hand to "help" Ship. Hand made files (not in the ship record) are
  left alone by Ship; keep it that way.
- Never capture the screen. Blender runs headless.

## 1. Read the project

1. `list_projects`: the project must have target `game`. If `siteRepoMissing` is true the game
   repo was picked on another computer: set it in step 2 or ask where it is.
2. `get_game` `{ project }`: settings (format, model folder, texture format and cap, `dropMaps`,
   bake, `assetFolders`, budgets per folder) and every model against its folder's budget.
3. `get_pipeline` `{ project }`: steps, validation issues, the run in progress, the latest runs.
   Game steps: Build models, Bake light (off by default), Export, Game export, Ship.
4. `get_project` or `list_assets`: assets, versions, stage, which are imported.

## 2. Settings

`set_game_settings` `{ project, repo?, format?, modelsDir?, textures?, textureCap?, dropMaps?,
bake?, budgets?, assetFolders?, template? }`. Fields left out keep their value. The answer returns
the settings like `get_game`. A value out of range is refused and nothing changes.

`template` is `prop-set`, `character`, `level-kit`, `level-kit-obj`, `web-game`, `godot`, `unity`,
`unreal`, `mobile`, `vr-standalone` or `game-jam`, alone or with `repo`. It fills the format, the
texture format, size cap and map dropping the template names (the first four leave those alone) and
its folder budgets, and nothing else. `get_game` lists every template with its numbers under
`templates`; pass `previews` (ids, or `"all"`) to add what each would change in this game. It adds
no assets and moves no file.

| Field          | Values                                                                                                                           |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `repo`         | Absolute path to the game repo: a git checkout or a worktree. Saved for this computer only.                                      |
| `format`       | `glb` or `obj`, for models made in Maquette. Imported models keep theirs. Match what the engine's loader reads.                  |
| `modelsDir`    | The model folder inside the repo, for example `assets/models`.                                                                   |
| `textures`     | `png`, `jpg` or `keep`.                                                                                                          |
| `textureCap`   | `256`, `512`, `1024`, `2048` or `4096`: the longest edge for models in folders WITHOUT a budget. A folder's budget size wins.    |
| `dropMaps`     | `true` leaves normal and metal-roughness maps out (the engine lights flat or packs its own); `false` (default) keeps them.       |
| `bake`         | Whether the pipeline runs Bake light. Off for games that light at runtime.                                                       |
| `budgets`      | The WHOLE table, one row per folder: `{ folder, triangles, texture }` (triangles at least 100, texture at least 64). Read first. |
| `assetFolders` | `{ "<asset id>": "kit" }`: the asset's own folder inside the model folder, no `..`. `null` or `""` puts it back.                 |

Ask the person for format, texture and `dropMaps` choices if the engine is unknown; check what the
repo's loader reads before assuming. KTX2, meshopt and Draco are never used in game export.

A moved asset (`assetFolders`) is written at the new path on the next Ship and its old file is
removed only when Maquette wrote or imported it.

## 3. Import the game's own models

1. `scan_game_models` `{ project }`: every model in the model folder (up to four folders deep)
   with triangles, textures, size, its folder's budget check and the asset it was imported as.
2. Tell the person which are over budget. Import the ones Maquette should look after:
   `import_game_models` `{ project, files }` with `files` relative to the model folder as the scan
   lists them (`props/brazier.glb`). Each becomes an asset whose version 1 is the repo file and joins
   the ship record. Models left out stay hand made. An import changes nothing in the game repo.
3. Previews render in the background; wait for them before judging pictures.

## 4. Budgets, fix-ups, size, clips

- Over budget: triangles and the largest texture are checked against the folder's budget (nearest
  folder above if none). Decimate and Shrink textures fix-ups come from `add_fixups` `{ project,
asset, fixups: [{ op: "decimate", triangles }, { op: "shrink-textures", maxSize }] }` (other ops:
  `rotate` with `x`, `y`, `z` degrees, `apply-transforms`, `remove-loose`, `merge`, `split-parts`,
  `origin-bottom`; one of a kind replaces). `preview_fixups` runs them
  headless on a copy and returns the numbers before and after each: run it before relying on a
  fix-up. `list_fixups` and `remove_fixups` `{ project, asset, ops | all }` read and undo them.
- A rigged model never gets Decimate (it would tear skin weights and shape keys). Over its triangle
  budget it needs a hand reduction in Blender or a new generation; Shrink textures still applies.
  For a rig that looks wrong, use `maquette-character-rig` first.
- Real size: `get_real_size` `{ project, asset }` then `set_real_size` `{ project, asset, size:
{ axis, value, unit } }` with `axis` one of `height`, `width`, `depth`, `longest` and `unit`
  `cm` or `m`; `size: null` clears it. The model is scaled uniformly so that side measures the
  size. Use the game's own unit: ask what one engine unit is before setting values.
- Clips: each rigged model keeps only its own clips in its `.glb`, never another rig's. Bake library
  clips with `use_animations` (`maquette-character-rig` step 8). A game export keeps every clip in
  the model file, so there are no ship switches. Clips keep their frame rate (30 fps stays 30).
- LOD levels are shown in the panel but NOT built yet. Do not promise them; if the person needs
  LODs, say it is not available and offer separate decimated assets instead.
- `compare_versions` `{ project, asset, left, right }` shows two versions part by part when you need to
  prove a fix-up kept what it should.

## 5. Plan, then run

1. `get_ship_plan` `{ project, repo? }` reads only. It lists new, replaced (marked when it
   replaces the imported original), unchanged and hand made files, the old files a moved asset
   leaves (removed), the checks (budgets, no KTX2 or meshopt), the branch, the ship record size,
   and `uncommitted`: how many files git status lists in the repo. With `repo` it plans against
   another checkout or worktree for this look only; nothing is saved.
2. If `uncommitted` is above 0, tell the person: Ship's change is easier to review and undo when
   the repo is clean. Do not clean it yourself.
3. To ship into a worktree without changing the project: pass the worktree path as `repo` to BOTH
   `get_ship_plan` and `run_pipeline`. It applies to that one plan or run.
4. Build first: `run_pipeline` `{ project, buildOnly: true, waitSeconds? }` builds everything
   and writes nothing to the repo; it never waits for approval. Check the run with `get_run`.
5. Ship: `run_pipeline` `{ project, repo?, waitSeconds? }` (also `from` a step id, or `force`
   to ignore the cache). It waits up to `waitSeconds` (20 default, 120 at most), then answers with
   the status. When it answers `waiting for approval`, STOP starting runs. Tell the person to
   Approve and ship (or Deny) in the app, with the plan's counts.
6. Poll `get_run` `{ project, number?, runId?, logLines? }` every 15 to 30 seconds until the status
   is `done`, `failed` or `cancelled`. `waiting for approval` means the person has not answered.
   Do not poll faster than that and do not give up on your own; if it stays waiting, say so and
   ask the person.
7. A failed run: read the log (`logLines` up to 400), fix the cause (a refused budget, a missing
   tool in `get_pipeline` validation) and run again from the failing step with `from`.

## 6. Check the export in the repo

After `done`, check the game repo (read only):

1. `git status` and `git diff --stat` in the repo or worktree: the files should match the plan
   (new, replaced, removed). Nothing else may change.
2. Each written model is a plain file in the configured format at its path, textures inside a
   `.glb` or beside an `.obj`/`.mtl`, within the folder's triangle and texture budget.
3. No KTX2, meshopt or Draco extensions; with `dropMaps` true, no normal or metal-roughness maps.
4. Hand made files are untouched; a moved asset's old file is gone only if Maquette wrote it.
5. `get_ship_plan` again: everything now reads `unchanged`, and the ship record size is what the
   run reported.

Report the file list, any model over budget that shipped anyway, and what the person should
review before committing.

## 7. Models for another engine, not this repo

To give finished models to a friend or another engine without shipping into a game repo, use an
export pack: `export_pack` `{ entries | collection | project, preset?, settings?, dryRun? }` (give
one source; library entry ids come from `list_library`). Run `dryRun: true` first and read the
plan. It writes OBJ, GLB or both with a README, a manifest and a contact sheet inside the
workspace, then zips it. Each model is opened again in headless Blender and checked. Poll
`get_pack` `{ id }` (`list_packs` lists presets and recent packs). A warned or failed model waits
for `decide_pack` `{ id, choice, key?, triangles? }`: `raise` a budget (with `key` and
`triangles`), `keep` a warning, export `again`, `zip-passing`, `zip-all`, `stop` or `resume`. Say
what each choice costs the pack before you answer for the person. Mixamo clips are never put in a
pack: the bones stay and the clips are left out. A pack spends nothing and writes only inside the
workspace; it never touches a game repo.

## The ship record

Maquette keeps a record of every file it wrote or imported. Ship writes only files in the record
or new ones, removes only a moved model's old file that is in the record, and never touches a file
that is not (hand made files are shown as left alone, also when an export would land on one).
Do not edit the record, and do not delete files in the repo to force a clean plan.

## Stop conditions

- Stop at `waiting for approval`, on a Deny, and on any failed run you cannot explain.
- Stop and ask before: changing `repo` of a project to a worktree for good, setting a budget lower
  than the current models, or dropping maps for a game you have not seen the shaders of.
