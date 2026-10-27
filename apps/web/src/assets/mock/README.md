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

`src/ipc/mock/install.ts` registers every file here under the mock workspace
paths, so new renders only need the right file name.

## License

Apart from the generated Motion demo models above, these images and models
are Frostveil artwork. They are the author's own work and all rights are
reserved. They are bundled only so mock mode looks like a real workspace, and
they are not covered by the license of this repository or by any open license
it may adopt in the future. Do not reuse, redistribute or train on them.
