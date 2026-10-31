# Mock assets

Images and models that mock mode (`pnpm dev` in a plain browser) shows for the
sample "Frostveil" workspace. Frostveil is the author's own game: a frozen rune
lake at night with a veil gate, an observatory arch, crystal lanterns, guardian
statues, a rune obelisk and Elsheria, its swordswoman. Its props and characters
came through the Maquette pipeline, so the sample shows real work.

- `*.webp`: 256 px studio thumbnails for the Board cards and Inbox rows. They
  are the renders the thumbnail service made for the Frostveil project (the
  same headless Blender render the app makes), converted to WebP.
- `concepts/*.webp`: the concept images of the Lab's sample directions (arena
  and character sheets from the game, the kit and prop sheets and the object
  refs of the Frostveil project), 960 px wide, object refs 512 px.
- `site/hero.webp`: the landing picture of the Page view's fake site, cropped
  from a Frostveil key art.
- `models/elsheria.glb`, `veil_gate.glb` and `ice_crystals.glb`: the models the
  Inbox preview, the Inspector and the Page view load. They are compressed
  copies of the Frostveil models, made with gltf-transform: simplified,
  textures at 512 to 1024 px in KTX2 ETC1S, meshopt geometry. Elsheria keeps
  her 119 bone rig and two clips (`draw_sword_1`, `spellblade_idle`) for the
  Motion view. They also exercise the decoders the preview sets up.
- `models/clay_figure.glb`, `clay_figure_rest.glb`, `switch_cabinet.glb` and
  `turntable.glb`: plain primitives for the Motion view's `?motion=` demo
  (a rigged figure with three clips and without, a cabinet whose door and
  breakers states move, a turntable with one clip). They are generated
  headless with
  `crates/maquette-core/src/motion/fixtures/motion_models.py` and are not part of
  the reserved artwork below.
- `models/frost_lantern.glb`, `frost_lantern_split.glb` and `rune_chest.glb`:
  plain primitives for the Parts demo (`?parts=demo`): a frost lantern as one
  mesh the way a generator sends it, the same lantern cut into five parts as
  the split fix-up leaves it, and a rune chest as one mesh. They are generated
  headless with `crates/maquette-core/src/parts/fixtures/parts_models.py`
  and are not part of the reserved artwork below either.
- `models/frost_lantern_tripo.glb`, `sigil_gate_frame.glb`,
  `sigil_gate_door.glb`, `sigil_gate_steps.glb` and `sigil_gate.glb`: the
  same kind of primitives for splits on a provider and Assemble one prop: the
  lantern's parts the way a Tripo split names them, untextured, the three
  Sigil gate parts as models of their own, and the gate assembled from them
  by `crates/maquette-core/src/parts/assemble.py`. Generated the same way.
- `avatar/sheet_before.webp` and `sheet_after.webp`: the rest pose sheets of the
  Avatar view, before and after a normalise. They are the sheets
  `crates/maquette-core/src/avatar/avatar_normalize.py` renders headless, made
  from the box humanoid test fixture in an A-pose facing +X, 1200 px wide, and
  are not part of the reserved artwork below.
- `avatar/reweight_before.webp` and `reweight_after.webp`: the Avatar view's freed arms
  sheets, before and after a reweight. They are the sheets
  `crates/maquette-core/src/avatar/avatar_reweight.py` renders headless, made from the box
  humanoid test fixture with a strip of cloth on its left arm (`cape_strip`), 1200 px wide, and
  are not part of the reserved artwork below either.
- `avatar/springs_candidates.webp`, `springs_before.webp` and `springs_after.webp`: the
  Springs tab's sheets (the parts found, the chains, the sway test). They are the sheets
  `crates/maquette-core/src/avatar/avatar_springs.py` renders headless, made from the box
  humanoid test fixture with a strip hanging from its head down its back, and
  `avatar/box_humanoid_springs.vrm` is that fixture written with its spring chain, which the
  Preview tab loads once the chains are approved. Both come from
  `MAQUETTE_AVATAR_OUT=<folder> cargo test -p maquette-core find_chain_and_write_springs_in_blender -- --ignored`
  (sheets converted to WebP), and are not part of the reserved artwork below either.
- `avatar/box_humanoid.vrm`: the `.vrm` every mock avatar build "writes", which
  the Avatar view's Preview tab loads. It is generated: the box humanoid test
  fixture written as VRM 1.0 by the Rust writer, with
  `MAQUETTE_MOCK_VRM=apps/web/src/assets/mock/avatar/box_humanoid.vrm cargo test -p maquette-glb writes_the_mock_vrm -- --ignored`.
  It is not part of the reserved artwork below either.

`src/ipc/mock/install.ts` registers every file here under the mock workspace
paths, so new renders only need the right file name.

## License

Apart from the generated Motion and Parts demo models, the avatar sheets (rest pose and freed arms) and the box humanoid VRM above, these images and models are Frostveil
artwork. All rights are reserved; they are not covered by the code license. See
[ARTWORK.md](../../../../../ARTWORK.md) at the repository root.

- `renders/*.webp`: what the Render tab shows: real headless renders of a lantern made by `crates/maquette-core/src/renders/render_styles.py`, turntable frames every eighth of the turn at 480 px, the four looks at 720 px.
