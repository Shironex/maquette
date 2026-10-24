# Mock assets

Images and models that mock mode (`pnpm dev` in a plain browser) shows for the
sample "Moon Island" workspace: a small floating island diorama with a shop
house, a vending machine, cherry trees, stone lanterns, a far spire and a
character.

- `*.webp`: 256 px studio thumbnails for the Board cards and Inbox rows. They
  were rendered once with the thumbnail service
  (`cargo run -p maquette-core --example render_thumbnail`), the same headless
  Blender render the app makes, then converted to WebP.
- `concepts/*.webp`: the concept images of the Lab's sample directions,
  960 px wide. They were generated with the Codex CLI, the same way the Lab
  makes them, then scaled down.
- `models/*.glb`: the models the Inbox preview loads. They use meshopt
  geometry and KTX2 textures, so they also exercise the decoders the preview
  sets up.

`src/ipc/mock/install.ts` registers every file here under the mock workspace
paths, so new renders only need the right file name.

## License

These images and models are reserved artwork. All rights are kept by their
author. They are bundled only so mock mode looks like a real workspace, and
they are not covered by the license of this repository or by any open license
it may adopt in the future. Do not reuse, redistribute or train on them.
