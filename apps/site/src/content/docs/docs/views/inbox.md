---
title: Inbox
description: Models from Tripo, Meshy or your own files arrive here. Maquette checks them, offers fix-ups and moves the keepers into a project.
sidebar:
  label: Inbox
---

Maquette watches the workspace `inbox/` folder for `.glb` and `.gltf` files. Each file is read in Rust: triangles, meshes, materials, textures and their sizes, skins and joints, animations.

## Checks

The checks compare that with your budgets: triangles, texture size, a Mixamo rig for characters, PBR maps, separate objects merged into one mesh (for a prop, One mesh, no parts), an origin away from the bottom centre, several objects that could be one, and hair or tail vertices weighted to finger bones. Two of them are heuristics: merged objects and weighted to finger bones. Draco or meshopt compressed geometry skips both.

A check that a fix-up answers offers it as a button: Decimate to 60k, Shrink textures to 2048 px, Remove loose parts, Split into parts, Origin to bottom centre or Merge objects.

### VRM readiness

A rigged character also gets a VRM row: how far it is from a VRM 1.0 avatar, read without Blender. It never fails a model, since VRM is an export a character may never need: it passes when a VRM file could be written from the model as it is, and warns otherwise, naming every finding with the blockers first.

- **Bones.** The joints are mapped onto VRM's 55 humanoid bones by name: VRM's own (and VRoid's `J_Bip_` names), Mixamo, Unreal, and Tripo's own rig names (a guess no real Tripo rig has confirmed yet). The 15 VRM requires (hips, spine, head, and both upper legs, lower legs, feet, upper arms, lower arms and hands) must all be there, each bone must sit under the bone VRM puts above it (other nodes may sit between), and the upper chest and each finger segment need the bone above them.
- **Rest pose.** VRM's T-pose faces +Z with the character's left on +X, arms level (within 3 degrees), soles on the ground, the hips on the centre, both sides mirrored, and every node scaled the same on each axis and never negatively. Facing, the arm angles and the scales block; the ground, the centre, the symmetry and a scaled node only warn.
- **Notes.** The bones it maps, the optional ones it lacks (eyes, jaw, fingers), clips the export drops, and node scales off 1 by float noise, which the writer snaps.

A typical Tripo character faces +X and stands in an A-pose with its arms about 50 degrees below level: both block until its rest pose is normalised.

## Preview

The preview stands every model in the same studio: a graphite backdrop, a soft floor with a contact shadow and warm and cool lights. Pick Shaded, each part in its own colour (the split copy once a Split into parts preview ran), wireframe over clay, a numbered UV checker, or the skeleton drawn over a ghost of the mesh. It reads meshopt geometry and KTX2 textures.

## Real size and fix-ups

Each asset can have a real size and a list of fix-ups, both optional. A size is one side of the real object (height, width, depth or the longest side) in cm or m. The fix-ups run in a fixed order: Rotate, Apply transforms, Remove loose parts, Merge objects, Split into parts, Decimate, Shrink textures and Origin to bottom centre. Rotate turns a model that came in leaning or lying down upright: degrees about x, y and z (up to 360 either way), turned about the middle of its bounds before anything else runs, so the size and Origin to bottom centre measure the upright model. Split into parts is on its own page: [Parts](/docs/views/parts/). Nothing touches the inbox file or the asset's versions: the [Build scene](/docs/pipeline/build-scene/) step applies them headless on a copy.

You can also skip the bake for an asset and pad its texture edges. A model accepted from the Inbox skips the bake and pads by default, since provider scans cut their textures into many small islands.

## Accept and redo

Accept to Blender moves the file into the project's `models/` folder as a new asset version. By default it goes to the asset the model was sent for, or the one a download was attached to. A model accepted for an asset with a By hand send still waiting also closes that send, as done with the note "Model added to the asset, no longer waiting"; a Tripo or Meshy send of the asset runs on. Redo sends an inbox model's asset again, prefilled from its send; the earlier model stays in the inbox so you can compare and accept one. A model that came as one mesh can also go to Tripo or Meshy to be split into parts from its One mesh, no parts check, and the split comes back as another inbox model; a Meshy split's texture bakes back once it is accepted. See [Parts](/docs/views/parts/#split-on-tripo-or-meshy).

## Downloads from By hand sends

While a By hand send waits, the Downloads watch (Settings > 3D providers) looks at new `.glb`, `.fbx` and `.zip` files. A file named for its send (`maq_<asset>_v<n>_<provider>`) goes into the inbox by itself. Anything else waits in **New downloads**, opened from its notice, however long after its handoff it came.

Each of those is matched by its picture too. Blender renders it headless from the front on a plain ground (one file at a time, kept by its content so it renders once), and every waiting By hand send is ranked by how much its front ref looks like it: how much the two outlines cover the same area once both are scaled to one square, how much colour they share (by hue, and by lightness for greys, so softer light still matches) and how close their proportions are. The file name only breaks ties. A clear best, at least 60% alike and ahead of the next, is suggested with its score; nothing is ever attached by itself.

- **Needs a look**: the picture looks like one waiting send's, the name is close to one (a site's batch export such as `lantern post 3d model.glb`), or it is the only send waiting. The card shows the download's front render next to the send's front ref, how alike they are ("88% alike"), the file name and why it matched. **Attach to Lantern post** takes it, the other waiting sends are listed after it, ranked, to attach to instead, **Pick another asset** chooses any asset, and **Not this one** leaves the file alone and never asks about it again.
- **Which asset is this?**: files it could not place, with the closest send when the picture was read. **Attach to an asset** opens a searchable list of the project's assets, the ones a By hand send waits on first, best fit first with how alike each is, each with its ref picture and state. Type to search, use the arrows and press Enter; the list moves on to the next file, so a batch goes quickly. Esc closes it.

Without Blender, or for an FBX, the name alone ranks the sends.

**Add a model file as the next version...** in a card's menu on the Board does the same for a file you made or fixed yourself, whether or not a send waits: pick a `.glb`, `.gltf`, `.fbx` or `.obj` and it is copied into the inbox for that asset (the others are turned into a GLB by headless Blender first). A file already in the inbox or in a project's `models` folder is refused.

Attaching sends the model to the inbox for that asset, as a file named for its send would go: for the asset's newest waiting By hand send, or aimed at the asset when none waits. The usual checks run, and the real size and fix-ups draft is there as for any inbox model. Accept goes to that asset by default and closes its By hand sends still waiting. The downloaded file stays where it is.
