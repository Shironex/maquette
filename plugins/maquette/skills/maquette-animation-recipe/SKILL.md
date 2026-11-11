---
name: maquette-animation-recipe
description: Change a clip on a rigged Maquette model without redrawing it: make an animation recipe from a starter, a library recipe, a clip or the rest pose, fill its changes and blanks, bake it in headless Blender, read the review numbers and ask the person to add it as the model's next version or save it to the recipe library. Use when asked to make a model wave, reach for a prop, look at a point or lean, to adjust an idle or walk, to import or export a recipe, or to reuse a recipe in another project. Not for rig or skin weight fixes (maquette-character-rig).
---

# Maquette animation recipe

An animation recipe is a base clip (or the model's rest pose) plus changes on humanoid bones: reach,
nudge, look-at and wave. One small file per recipe in the project's `recipes/` folder, never keys or
a mesh, so it can be shared. Baking solves the changes exactly in headless Blender into one new clip
on a staged model. A person reviews the bake and adds it as the model's next version. Tools are
Maquette MCP tools (`/docs/agents/mcp-tools/`); `project` is the id `list_projects` returns and
`recipe` the id `list_recipes` returns. The guide is `/docs/guides/animation-recipes/`.

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list; it is short, and `detail: true` gives every row and gate. No project to work in: `create_project` (`target` is `website` or `game`) makes one and spends nothing. No tool removes a project; a removed project's id fails with not-found, and only a person restores it. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.
Settings > Agents can turn tool groups off, all but start and project: their tools are missing from
your list (or answer code `group-off`), and `get_started` names them in `toolGroupsOff`. Any tool this
skill names may be off. When a step needs a missing tool, do not work around it: ask the person to
turn its group on in Settings > Agents, then go on.
A project's planning canvases (the plan group) hold the person's notes, links and plans: `list_canvases`
lists them and `get_canvas` reads one, so read the canvas the person points at before you plan for them.
Agents never delete a person's nodes: ask with `propose_removal`, and only the person answers.
A website project's camera plan (`get_camera_plan`) is tried there until the person presses Apply
in the app: agents may propose a tour, never apply one.

## Hard rules

- Only a person adds a bake as a version, saves a recipe to the library (a starter included),
  removes library recipes or revisions, imports into the library in the app, or exports outside
  `exports/recipes`. You ask; you never do these by another route.
- Never add a staged bake with `add_model_file`: it is refused (`staged-bake`). A bake of a Mixamo
  clip counts as Mixamo's and is kept out of packs and the clip library; `add_recipe_bake` keeps that.
- Never capture the screen. Blender runs headless only. The person looks at the review in the app.
- Send `expected` (the `version` from `get_recipe`) with every `update_recipe` that sends changes,
  blanks or clear. The lists replace the whole list: send every entry you keep.
- A `warn` in the review never blocks an add, but tell the person what warned.
- Do not poll in a tight loop: wait `pollEverySeconds` when a tool gives it.

## 1. Pick a start

1. `list_library_recipes` `{}`: `starters` (read only, ids `starter:...`) and the library's recipes.
   Each starter has `about` and a `base` with `names` (the clip names it tries) or `rest`, and
   `library`: `found`, `missing` or `rest` on this computer. `get_library_recipe` `{ entry }` reads one in full; a starter id reads that
   starter. The five starters: `starter:wave-hello` (on the rest pose), `starter:look-at-point`,
   `starter:reach-for-prop`, `starter:lean-in` (on an idle) and `starter:point-while-walking`.
2. A starter or library recipe fits: `use_recipe` `{ project, entry, revision? }`. It copies it
   into the project as a new recipe. A starter id has no library link and `revision` must be left
   out. A prop blank whose asset the project lacks opens, and `notes` says so. A base clip that is
   missing still makes the recipe; the starter keeps its first clip name, so importing a clip of
   that name (`import_animations`) fixes it.
3. Nothing fits: `create_recipe` `{ project, clip | restPose, name? }`. Give `clip` (an id from
   `list_animations`) or `restPose: true`, one of the two. Rest pose is 60 frames at 30 fps, never
   Mixamo's. It starts with no changes and no blanks.
4. Or bring a file: `import_recipe` `{ project, path }` reads a `*.recipe.json` inside the
   workspace into a project (at most 256 KB, 32 changes, 16 blanks). Without `project` it is
   refused (`library-import`): import into a project, then `propose_recipe_save`.

## 2. Read and edit

1. `list_recipes` `{ project }` and `get_recipe` `{ project, recipe }`: the base with `library`
   (`found`, `hash-differs`, `missing` or `rest`), `blanks`, `changes`, `warnings`, `version`, and
   `library` link and `saveWaiting` when they apply. A missing base needs `import_animations`
   before it can bake.
2. `update_recipe` `{ project, recipe, expected, name?, changes?, blanks?, clear? }`. Everything is
   checked before anything is written.
   - Kinds and bones: `reach` (`leftHand`, `rightHand`, `leftFoot`, `rightFoot`, to a blank),
     `look-at` (`head` or `neck`, to a blank, clamped to 80 degrees), `nudge` (any humanoid bone,
     `rotation` in degrees and `offset` in metres), `wave` (arm bones, `cycles` and `amount`).
   - `start` and `end` are shares from 0 to 1 of all the clips' frames end to end (on one clip,
     of its length), `weight` from 0 to 1.
   - A blank is a `prop` (an asset id from `list_assets`) or a `point` ([x, y, z] in metres in the
     model's own frame: X to its left, Y up, Z ahead). Fill every blank the changes aim at.
   - `clear` empties a prop blank whose asset is gone for good.
3. Joined clips and base edits also go through `update_recipe`: `clips` (base first, each a
   library `clip` or `keep` of an index, with `trim` [in, out] and `blend`
   `{ frames, curve, lineUp }`), `time` (`retime`, `freeze`, `hold-over` on a clip's own frames
   before its trim) and `mirror`. They bake, and so do pins, pose keys and grips. `frames` in
   `get_recipe` is how long the bake runs.
   - A `pin` keeps `leftFoot`, `rightFoot`, `leftHand` or `rightHand` in place from `start` to
     `end`: where the bone is at its first frame, or on a blank in `target` (give a hand one).
     `easeIn` and `easeOut` are whole frames (3 unless set). A foot's `flat` is `toe-and-heel`
     (default), `heel` or `off`. Pins are solved last, after every other change.
   - Pose keys: `set_recipe_keys`
     `{ project, recipe, expected, layer?, mode?, weight?, set?, move?, remove? }` edits one pose
     key change by bone and frame (a new one without `layer`).
     A key sits on a clip's own frame (`clip`, `frame`); turns are degrees [x, y, z] about the
     model's own axes, added to the clip (`add`) or from the rest pose (`replace`), eased in by the
     later key (`smooth`, `in`, `out`, `linear`, `step`). `get_recipe_pose` `{ frame }` reads the
     blended turns at a frame and the joint limits they pass; `overLimits` only warns.
   - Grips: `list_grips` and `get_grip` read the grip library; `apply_grip`
     `{ project, recipe, grip, expected, revision?, hands?, layer? }` lays a revision on the recipe
     as its own copy with a link back, and with `layer` takes a newer revision into that grip change.
     `propose_grip_save` asks the person to save a pose key change's hands at one frame as a grip.
   - Changes not sent with `clips` move with their clip. Taking out a clip a change sits on, or
     moving a clip away from the clip a change runs across from, is refused (`clip-in-use`).
4. Tell the person in one line what the recipe does and which blanks you filled with what.

## 3. Bake

1. `bake_recipe` `{ project, recipe, expected? }` answers at once and the bake runs in the
   background. A recipe with open blanks is refused (`open-blanks`): fill them with `update_recipe`.
   It needs a rigged model in the project.
2. `get_recipe_bake` `{ project, recipe }`: poll `state` (`running`, `done`, `failed`,
   `cancelled`) and `progress`. When done, read `review`: hand or foot gap (2 cm), elbow or knee
   flip, pin drift (1 cm, per pin), foot slide (1.5 cm over the base clip's own), rig stretch
   (5 mm over the base clip's own), each `pass`, `warn` or `not-measured`, and how many warn.
   Also read `clipNote`: whether the clip replaced this recipe's older ones or took a `_2` name.
3. `stale` on a finished bake means the recipe or the model changed since: call `bake_recipe`
   again. A failed bake says why in `error`; fix that, then bake again.
4. The numbers measure only the four things above, at the reach ends and sampled frames. They do not
   say whether the motion looks right. Report them as they are and never call a warn a pass.

## 4. Ask the person to add it

1. `add_recipe_bake` `{ project, recipe }` answers `waiting for approval` with `pollEverySeconds`
   and `whereToApprove`. The person reviews the bake in the Motion view's Recipes tab and presses
   Add as vN there. Asking twice returns the same approval. A new bake cancels a pending ask.
2. Poll `get_recipe_bake` until `state` is `added` with `addedVersion`. A denied ask keeps the staged
   bake; ask the person what to change rather than asking again.
3. `bake-changed`: the bake under review was replaced. Read `get_recipe_bake` and ask again for
   the new one.
4. `stale-bake`: bake again with `bake_recipe`, then ask again.

## 5. Save to the library

`propose_recipe_save` `{ project, recipe, entry?, newEntry? }` asks the person to save the recipe as
the next revision of the library recipe it came from, or as a new library recipe. Nothing is saved
by the tool. Poll `get_recipe` until its `library` field names the saved revision. Revisions are
never changed and never reused. A recipe that holds what the newest revision holds is refused.
A starter is copied to the library the same way: use it in a project first, then propose the save.

`export_recipe` `{ project, recipe | entry, revision?, file? }` writes a new `.recipe.json` under
`exports/recipes` in the workspace, with no library link and no keys. A taken name gets `-2`.

## Errors and what to do

| Code             | What it means                                          | What to do                                                                     |
| ---------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------ |
| `moved`          | The recipe or library entry changed since you read it. | `get_recipe`, merge what changed, resend with the new `version` as `expected`. |
| `needs-version`  | Lists were sent without `expected`.                    | `get_recipe`, then send `expected`.                                            |
| `open-blanks`    | A blank is open.                                       | Fill it with `update_recipe`, then `bake_recipe`.                              |
| `staged-bake`    | A staged bake went to `add_model_file` or a download.  | Call `add_recipe_bake` instead.                                                |
| `stale-bake`     | The recipe or the model changed after the bake.        | `bake_recipe` again, then ask again.                                           |
| `bake-changed`   | The bake under review was replaced.                    | Read `get_recipe_bake`, ask again for the new bake.                            |
| `library-import` | `import_recipe` without `project`.                     | Import into a project, then `propose_recipe_save`.                             |

## Stop conditions

- A person-only step is next: say what you need from them and stop.
- Tool groups are off (`group-off`): ask the person to turn the animation group on in Settings > Agents.
- The base clip is missing from the library and you cannot import it: ask the person.
