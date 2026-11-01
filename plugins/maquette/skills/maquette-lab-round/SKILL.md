---
name: maquette-lab-round
description: Run a concept art round in Maquette's Lab for one or more assets (props, creatures, characters, scenes), from the style board to refs that are ready to send to 3D. Use when asked to make concepts, directions, object refs or extra views, to set the project style, or to clean a ref's background. Not for sending to Tripo or Meshy (maquette-by-hand-round).
---

# Maquette Lab round

Turns a brief into concept images, lets a person choose, and ends with each asset holding clean
front, left, back and right refs. Every tool below is an MCP tool of the Maquette app (see
`/docs/agents/mcp-tools/`); `project` is the id `list_projects` returns, `asset` the id
`list_assets` returns.

## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list. No project to work in: `create_project` makes one and spends nothing. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.

## Hard rules

- A person picks the concepts. Present the options (file paths, plus the star and pick state from
  `get_concepts`) and wait. Pick for them only when they told you to ("pick the best of each").
- Characters and creatures are generated WITHOUT held weapons, shields, lanterns, staffs or any
  other hand prop. A hand prop in the image gets fused into the hand in 3D and cannot be taken
  off. Props are separate assets, made on their own and attached to hand bones after rigging.
  Say so in every character prompt ("empty hands, relaxed fingers, no held items") and reject
  concepts that hold something.
- You cannot change the monthly OpenAI image cap. Never try `monthlyCapUsd`; it is refused.
- A round you start on OpenAI Images waits for a person to Approve in the app and answers
  `waiting for approval`. Do not start the round again; poll `get_concepts`.
- Never capture the screen. The tools return file paths; read the images from disk.

## 1. Read the cost state

1. `list_projects`, then `list_assets` for the project (and `get_project` if you need the target:
   website or game).
2. `get_codex_usage`: the five hour and weekly windows. If `known` is false, say so and carry on
   carefully. If the usage limit holds, jobs wait with the rest of the queue until the reset time
   it names; tell the person when, do not queue a big round into it. At 80 percent or more of a
   window, shrink the round (fewer directions, `count` 2 or 3) and say why.
3. `get_image_backend`: the backend per kind of job (`codex` or `openai-images`), the quality,
   the monthly cap, this month's spend and the estimate per image. Codex is the default for every
   kind. Use `openai-images` only when the person asked for it or the Codex limit blocks the work
   and they agreed. `set_image_backend` changes backends and quality only (quality `low`,
   `medium`, `high`; `high` costs about 20 times `low`). Tell them the estimate first.

## 2. Style board

1. `get_style_board`. If it has a line, a palette or images, reuse it and say so.
2. Otherwise `set_style_board` with the person's images and a style line: `line` (one line, 200
   characters at most, for example "painted stone, cool moonlight, chunky silhouettes"),
   `palette` (up to 6 `#rrggbb`), `images` (up to 4; each is a concept or ref of the project, as
   `get_concepts` lists it, or an absolute path inside the workspace). The call replaces the whole
   image list, so read the board first and send the full list.
3. Every new direction and object ref carries the board by default. `useStyle: false` skips it
   for one request. Results report `styleRevision`.

## 3. Concepts per direction

One `start_concepts` per direction: `{ project, direction, prompt, refs?, count?, aspect?,
useStyle?, backend? }`.

- `direction` is a short name ("Well, weathered"). Reusing a name adds images to it.
- `prompt` says what to draw, the view and the lighting: "a stone well with a wooden roof, three
  quarter view, plain light grey background, one object, no text". For a character or creature
  add the empty hands clause and a neutral standing pose with the arms slightly away from the
  body. For a set of things that must match, make one sheet first, then split it (step 5).
- `refs` are up to 8 images the person gave you (paths relative to the project folder).
- `count` 1 to 12, 4 by default; `aspect` is `16:9` (default), `3:2`, `1:1`, `3:4` or `9:16`.
  Pick `3:4` for a single character, `1:1` for a prop.
- Start one direction first when the cost state is tight, check its images, then the rest.

Then poll `get_concepts` (`{ project, direction? }`) until the jobs are done. It lists images with
stars and picks, plus queued, running, waiting and failed jobs.

## 4. Failed, slow or limited jobs

- A 429 from Codex runs again on its own after 3, 6 and 12 minutes. Just wait.
- `retry_concept_jobs` `{ project, jobs?, include? }` runs failed or cancelled jobs again with the
  same prompt, refs and file. `include: "held"` or `"all"` also runs the jobs waiting out a rate
  or usage limit now; only do that when `get_codex_usage` says the window has room.
  A job the app closed on comes back failed with "Interrupted by a restart": retry it.
- `retry_usage_limited_jobs` `{ project }` runs the jobs the usage limit holds, but only when a
  fresh read says the limit lifted; otherwise it says until when.
- `cancel_concept_jobs` `{ project, jobs? }` cancels queued, running, waiting or approval held
  jobs (every unfinished job without `jobs`). Use it when the person changes their mind.
- `prioritize_concept_jobs` `{ project, jobs? }` moves the project's queued jobs to the front,
  ahead of other projects.
- A job that fails with `refused: true` was refused by OpenAI's safety system: nothing was
  charged, and a retry sends the same prompt, so reword the direction instead.

## 5. The person picks

Show each direction's images with their paths. Optionally `star_concepts` for the ones they like.
Do not call `pick_concepts` until they chose.

`pick_concepts` `{ project, images: [{ file | direction + index, name?, kind?, star? }] }` (1 to 24) copies each image into `refs/` and makes or joins an asset at Refs. `name` is the asset's
name; `kind` is `prop`, `character` or `scene` for a new asset only. Calling again with an image
picked before makes no duplicate.

A concept that shows several objects (a kit sheet, a room): `make_object_refs`
`{ project, source, objects, notes?, kind?, aspect?, name?, useStyle?, backend? }`. `source` is
the concept path, `objects` up to 12 short names ("stone well", "rope bucket"), `notes` is keyed
by object name for long descriptions. Each object becomes one isolated image on a plain light
grey background in a new direction "<name> objects". The person picks those with `pick_concepts`
too. For a character sheet with held items, make the held items their own objects and the
character's refs without them.

## 6. Views for 3D

A send carries front, left, back and right, in that order, and leaves refs tagged `other` home.

1. `get_ref_views` `{ project, asset }`: the refs, their views, `tagged` (false means the views
   are read from the order and the first is the front), and the candidates already drawn.
2. `set_ref_views` `{ project, asset, refs: [{ file, view }] }` with `view` one of `front`,
   `left`, `back`, `right`, `other`. At most one ref per view. A ref left out stops being the
   asset's (its file stays in `refs/`). Send the complete list.
3. `make_other_views` `{ project, asset, views?, count?, useStyle?, notes?, backend? }` draws the
   EMPTY slots only (or the views you name), 1 to 4 candidates each, from the front ref. The
   front is never drawn. `notes` (one line, 400 characters at most) goes into every view prompt
   and survives a retry: "arms hang empty at the sides, no held items" or "the cloak is open at
   the back". Candidates land in the direction "<asset> views"; nothing reaches the refs yet.
4. A messy front ref (clutter, a cut-off edge): `clean_ref_background` `{ project, asset, file,
background?, pad?, extend? }`. `background` is `studio-grey` (default) or `transparent`;
   `extend: true` lets it draw what the frame cut off (leave it off unless the person wants it).
5. The person chooses candidates. Then `use_ref_candidate` `{ project, asset, file, view?,
replace? }` takes one in: a drawn view fills its slot (the ref it replaces moves to
   `refs/originals`, unless another asset or a send uses it; `replace: false` keeps it as
   `other`), a clean copy takes the original's place and view. Read its answer: `replaced` says
   what moved where. Re-run `get_ref_views` to confirm the final set.

## 7. Before handing on to a send

The send tools return `refWarnings` before anything is spent: small, cropped, busy background,
one view only, a side or back that looks different from the front. They never stop a send. If you
are about to send, read them (the first call of `send_to_3d` with `by-hand` is free and shows
them) and fix what the person cares about: clean the background, redraw the odd view with
`make_other_views` and `notes`, or accept the warning on purpose and say so.

## Stop conditions

- Stop and ask when a window is at its limit, the estimate passes what the person said, or a
  concept breaks the hand prop rule and a redo would cost more than they set aside.
- Stop after `waiting for approval`; report the estimate and poll, never repeat the call.
- Finish by listing, per asset: refs and their views, warnings left open, and what the next step
  is (`maquette-by-hand-round`).
