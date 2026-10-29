---
name: maquette-by-hand-round
description: Send assets to Tripo or Meshy BY HAND (a handoff folder, the person runs the provider's website) and land the downloaded models in Maquette. Use when asked to send refs to 3D, prepare a handoff, check what is waiting for a model, match downloads to assets, accept inbox models, read inbox checks, add a model to the library or redo a bad generation. Not for concept images (maquette-lab-round).
---

# Maquette by hand round

By hand means the person makes the model on the provider's website from the images and settings
card Maquette wrote, and exports a GLB. This skill writes the handoffs, then lands the downloads.
Tools are Maquette MCP tools (`/docs/agents/mcp-tools/`).

## Hard rules

- Every send to Tripo or Meshy over the API (`provider` `tripo` or `meshy`) spends credits and
  waits for the person to Approve in the app. Never start one unless they asked for an API send.
  The default of this skill is `provider: "by-hand"`, which spends nothing. After an API send
  answers `waiting for approval`, poll `get_provider_job`; never send again.
- `cancel_provider_jobs` cannot deny a send that waits for approval: the person denies it in the
  app.
- Never attach, accept or dismiss a download on a similarity score alone. LOOK at both pictures
  first (step 4).
- Never capture the screen. Renders are files written headless.

## 1. Choose what to send

1. `list_projects`, `list_assets` (`stage` filter optional): take assets that sit at Refs with a
   full set of views. If views are missing or refs are weak, run `maquette-lab-round` first.
2. `get_ref_views` per asset. A send carries front, left, back and right in that order.
3. `get_credits` `{ project }` shows balances and the ledger of paid sends, API spend and By hand
   subscription spend apart. It spends nothing; report it when the person asks about cost.

## 2. Write the handoffs

`send_to_3d` `{ project, asset, provider: "by-hand", service?, variant?, images?, mesh?, rig?,
overrides? }`

- `service` is whose website the settings card is for: `tripo` (default) or `meshy`.
- `variant`: `prop` or `character`, by default from the asset's kind.
- Tripo only: `mesh` is `hd-model` or `smart-mesh` (Smart Mesh P2.0); `rig` is `biped` (Mixamo
  skeleton), `quadruped`, `multi-legged` or `none`. Default is biped for a character that rigs,
  else none. Pick `none` for props and when the person will rig elsewhere; pick `multi-legged`
  for spiders and crabs, `quadruped` for four legged animals. A non-humanoid rig needs the
  person's attention afterwards: see `maquette-character-rig`.
- More than the front view goes as Multiview to 3D, each image in the slot of its view.
- `overrides` changes preset fields for this send only (an unknown name is refused with the valid
  list), for example `{ "faceLimit": 20000 }`.

For several assets use `send_batch_to_3d` `{ project, provider: "by-hand", service?, items: [{
asset, variant?, images?, overrides?, mesh?, rig? }], mesh?, rig?, overrides? }` (24 at most;
per item values win over the batch).

When the person asked for a Meshy API send (`provider: "meshy"`), always pass
`overrides: { "shouldRemesh": true, "targetPolycount": <budget> }` (15000 to 20000 suits most
props) and a short texture `prompt`. Without remesh Meshy returns its raw mesh, often over 500k
triangles, and decimating that in Blender crumples hard surface props. A batch takes one approval.

Read the answer:

- `refWarnings`: small, cropped, busy background, one view only, a view that differs from the
  front. They never stop a send. Tell the person which assets carry them; offer to fix first.
- `modelWarning`: the asset already has a model (for example from the library). The send makes
  its NEXT version and the library entry is unchanged.
- The handoff folder and the export name the site's file must carry:
  `maq_<asset>_v<n>_<provider>` (for example `maq_stone-well_v1_tripo.glb`). A download with that
  name lands in the inbox on its own.

Give the person a checklist per asset: open the folder, upload the images in the slots the card
names, apply the settings on the card (mesh, rig, face limit), generate, export GLB with the
expected name, save it where the Downloads watcher looks (Settings > 3D providers).

## 3. Read what is waiting

`list_handoffs` `{ project? }` reads only:

- `pending`: each By hand send waiting (job, asset, provider, export name, folder, front ref).
- `watcher`: null when no watcher runs. Then nothing arrives on its own; tell the person to turn
  it on or use `add_model_file` (step 5).
- `needsLook`: downloads whose picture looks like a waiting send's, whose name is close, or that
  came for the only send waiting. Each names the asset it looks like and why.
- `unmatched`: files it could not place.
- `arrivals`: every recent file and its state.
- Each `needsLook` or `unmatched` entry has `suggestions`, every waiting send ranked best first
  with `score` (0 to 1), `picture` (how alike the headless front render and the send's front ref
  are; null until the render is in or without Blender), `name`, and `image` (the ref compared);
  `render` is the download's front render PNG.

Exact `maq_...` names need nothing from you: they are in the inbox (`list_inbox`).

## 4. Match by picture, then look

Sites often name downloads after the caption ("stone well 3d model.glb"). For each file in
`needsLook` or `unmatched`:

1. Collect the pictures: the download's front render (`render` in its top suggestion, or make
   one: `render_thumbnail` `{ file }` for a model inside the workspace, which returns a PNG path;
   for a file outside it run
   `blender -b --factory-startup -P <skill>/scripts/render_front.py -- download.glb out front`),
   and the front ref of EVERY waiting send (`image` in the suggestions, or `refs/...` of the
   asset).
2. Build a contact sheet so the candidates sit side by side, with ffmpeg:
   `ffmpeg -y -i render.png -i ref_a.png -i ref_b.png -filter_complex
"[0]scale=-2:384[a];[1]scale=-2:384[b];[2]scale=-2:384[c];[a][b][c]hstack=inputs=3" sheet.png`
   First tile the download, then each candidate ref. Write sheets outside the workspace and the
   repo (a scratch folder).
3. LOOK at the sheet (read the PNG). Compare silhouette, proportions, colour, held items, and any
   feature that is unique to one asset. Say what matches and what does not.
4. Decide:
   - One clear match you can name a reason for: `attach_download` `{ arrival, project, asset }`
     (`arrival` is the download's `id` from `list_handoffs`). It answers with `inboxFile` and the
     `job` it went to.
   - Two assets look alike (two variants of one thing): ask the person. Do not guess.
   - Not a Maquette model or not one of these: `dismiss_download` `{ arrival }`, only after the
     person agrees; it never asks about that file again.
5. A download that never reached Downloads, or a file you or the person fixed elsewhere:
   `add_model_file` `{ project, asset, path, accept? }` with an absolute path to a `.glb`,
   `.gltf`, `.fbx` or `.obj`. A file already inside the inbox or a project's `models` folder is
   refused. It is copied, never moved. With `accept: true` it is accepted at once.

## 5. Check, then accept

1. `list_inbox`: kind, size, triangles, worst check, `aimedAt` for an attached file.
2. `get_inbox_checks` `{ id }`. What the common findings mean:
   - Textures at 4096 px: normal for a provider export. They shrink at export to the compression
     preset, so this is information, not a defect.
   - No PBR maps (no metal or roughness, no normal): normal for some provider modes. It only
     matters if the person wanted PBR; say so, do not redo for it.
   - Triangles over budget: a Decimate fix-up is offered; a rigged model never gets Decimate (it
     would tear the weights), the person reduces it in Blender or redoes with a lower face limit.
   - Merged objects, "One mesh, no parts" on a prop: a Split into parts fix-up can follow; not an
     error for a single object.
   - Rig is not Mixamo (characters), hair or tail weighted to finger bones, no clips: hand off to
     `maquette-character-rig` before accepting as final.
   - Origin away from the bottom centre, scale off: fix-ups (`add_fixups`, `set_real_size`) cover
     them.
3. Accept: `accept_inbox_model` `{ id, project, asset?, name? }`. Without `asset` it goes to the
   asset the model was sent for, or the one the download was attached to. It makes a new asset
   version and closes that asset's waiting By hand send. A Tripo or Meshy API send of the asset
   keeps running. Use `name` only when a NEW asset should be made.
4. A character with a rig problem: accept the original as v1 first, then continue in
   `maquette-character-rig` so v1 stays as a safe base.

## 6. Library

`add_to_library` `{ project, asset, name?, tags? }` copies the newest model, picture, refs, real
size, fix-ups and tweaks into the library when the asset should be reusable. The same model again
adds nothing; a newer model of an asset already there adds the next version. A DIFFERENT design
of an existing entry (another well, same name) gets its own entry: give it its own `name`, do
not stack it as a version. `list_library` `{ query?, tag? }` first, to see what exists.

## 7. A bad generation

`redo_inbox_model` `{ id, provider?, images?, overrides?, mesh?, rig? }` (`id` is an inbox model that came from a send, `sentFor` in `list_inbox`; `redo_inbox_models` takes `ids` of one project, 24 at most) sends the
asset again prefilled from its send. For By hand it writes a new handoff and spends nothing; for
Tripo or Meshy it costs credits and waits for approval. The earlier model stays in the inbox so
the person can compare. Before redoing, say what you would change (a cleaner front ref, an
added view, another `mesh` or `rig`, a lower face limit) so the redo is not the same send again.
The inbox has no discard yet: the losing model stays there until the person removes the file.

## Stop conditions

- Stop at any `waiting for approval` and report; the person answers in the app.
- Stop when a match is not clear from the pictures. Ask with the sheet's path.
- Finish by listing: what was sent, what landed (asset, version, inbox warnings), what still
  waits (`list_handoffs`), and anything that needs `maquette-character-rig`.
