/**
 * The page's words, kept apart from layout so the tour panels, the timeline
 * and the pipeline section stay in step. Plain and short; no dashes.
 */
import { withBase } from '../lib/with-base';
import type { StopId } from '../scene/manifest';

export interface TourStop {
  id: Exclude<StopId, 'hero'>;
  number: string;
  title: string;
  line: string;
  /** What the camera looks at, under the timeline dot. */
  shot: string;
}

export const TOUR: readonly TourStop[] = [
  {
    id: 'concept',
    number: '01',
    title: 'Concept',
    line: 'A brief and refs become Codex image jobs.',
    shot: 'close on the paper sketch',
  },
  {
    id: 'refs',
    number: '02',
    title: 'Refs',
    line: 'Picks go to refs and move on the Board.',
    shot: 'pinned prints',
  },
  {
    id: 'model',
    number: '03',
    title: '3D model',
    line: 'Tripo, Meshy, or by hand.',
    shot: 'the clay maquette',
  },
  {
    id: 'blender',
    number: '04',
    title: 'Blender',
    line: 'Headless build, bake, export.',
    shot: 'wireframe sweep',
  },
  {
    id: 'web',
    number: '05',
    title: 'Web',
    line: 'KTX2 and meshopt, checked in the Inspector.',
    shot: 'island on the turntable',
  },
  {
    id: 'ship',
    number: '06',
    title: 'Ship',
    line: 'Copied into your site repo.',
    shot: 'pull back to the bench',
  },
];

export interface Shot {
  src: string;
  width: number;
  height: number;
  alt: string;
  tool: string;
  caption: string;
}

const screen = (name: string, width = 1440, height = 900) => ({
  src: withBase(`/scene/screens/${name}.webp`),
  width,
  height,
});

export interface Stage {
  id: TourStop['id'];
  title: string;
  body: string;
  /** The one line the phone layout shows instead of body and screens. */
  short: string;
  note?: string;
  chips: readonly string[];
  shots: readonly Shot[];
}

export const STAGES: readonly Stage[] = [
  {
    id: 'concept',
    short: 'A brief and refs become Codex CLI image jobs, four at a time.',
    title: 'Concept',
    body: 'Write a brief, drop in up to 8 reference images and pick how many concepts you want. Each image is one Codex CLI job, four at a time. Star the good ones, pick the keepers, send picks to Refs.',
    chips: ['codex exec', '1 to 12 images', 'contact sheet'],
    shots: [
      {
        ...screen('lab'),
        alt: 'The Concept lab with a brief on the left and six generated concepts, one picked.',
        tool: 'Lab',
        caption: 'Moon island porch, 6 concepts, 1 picked',
      },
    ],
  },
  {
    id: 'refs',
    short: 'Picks land in refs/ and the card moves on the Board.',
    title: 'Refs',
    body: "Picked images are copied into the project's refs folder and the asset moves to Refs on the Board. From here every card shows where it stands: queued, generating, waiting for your model, done.",
    chips: ['refs/', 'Board'],
    shots: [
      {
        ...screen('board'),
        alt: 'The Board with six columns, Concept to Ship, and asset cards with thumbnails and status.',
        tool: 'Board',
        caption: 'Moon Island, 8 assets',
      },
    ],
  },
  {
    id: 'model',
    short: 'Tripo, Meshy, or by hand with a settings card and no credits.',
    title: '3D model',
    body: 'Send refs to Tripo or Meshy from your preset, or make the model yourself in their web app. By hand spends nothing: Maquette writes a folder with the images and a settings card, then waits for the GLB.',
    note: 'A job sent before you quit keeps polling after a restart instead of paying again.',
    chips: ['Tripo', 'Meshy', 'By hand'],
    shots: [
      {
        ...screen('send-tripo', 540, 900),
        alt: 'Send to 3D panel with Tripo selected, reference images and the prop preset.',
        tool: 'Send to 3D',
        caption: 'Tripo, key from the keychain',
      },
      {
        ...screen('by-hand', 540, 900),
        alt: 'By hand card with three steps and every Tripo setting to copy.',
        tool: 'By hand',
        caption: 'no API call, no credits',
      },
    ],
  },
  {
    id: 'blender',
    short: 'Headless build, light bake and GLB export, cached per step.',
    title: 'Blender',
    body: 'Blender runs headless. Starter scripts build the scene from your models, bake light into a lightmap per object and export one GLB per model. A step whose inputs did not change comes from the cache.',
    chips: ['blender -b', 'build_scene.py', 'bake_light.py', 'export_web.py'],
    shots: [
      {
        ...screen('pipeline-running'),
        alt: 'Pipeline runner: six steps, two cached, Export GLB running at 73 percent.',
        tool: 'Pipeline',
        caption: 'run 25, Export GLB at 73%',
      },
    ],
  },
  {
    id: 'web',
    short: 'KTX2 textures and meshopt geometry from per model presets.',
    title: 'Web',
    body: 'Compress runs one gltf-transform chain per model: KTX2 textures in ETC1S or UASTC, meshopt geometry. Presets live per model, and a locked row keeps its values and says why.',
    chips: ['KTX2', 'meshopt', 'toktx', 'gltf-transform'],
    shots: [
      {
        ...screen('pipeline-compress', 1220, 420),
        alt: 'Generated artifacts with raw and compressed sizes, the live log, and per model compression presets.',
        tool: 'Pipeline',
        caption: 'island.glb 4.6 MB to 1.1 MB',
      },
    ],
  },
  {
    id: 'ship',
    short: 'Posters per camera stop, then GLBs copied into your site repo.',
    title: 'Ship',
    body: 'Capture posters renders one poster per camera stop. Then ship.mjs copies the compressed GLBs and the posters into the site repo you set for the project.',
    chips: ['render_stops.py', 'ship.mjs', 'public/models'],
    shots: [
      {
        ...screen('pipeline-done'),
        alt: 'A finished run: every step done or cached, Ship included.',
        tool: 'Pipeline',
        caption: 'run 23, 4 done, 2 cached',
      },
    ],
  },
];

export const SHOTS = {
  inbox: {
    ...screen('inbox'),
    alt: 'Model inbox: a character with one failed check and two warnings before Accept to Blender.',
    tool: 'Inbox',
    caption: '4 models waiting, 1 failure and 2 warnings on Character',
  },
  compare: {
    ...screen('inspector-compare'),
    alt: 'Scene comparison of v6 and v7 with a split slider and five camera stops below.',
    tool: 'Compare',
    caption: 'v6 baked against v7, camera stops from stops.json',
  },
  budgets: {
    ...screen('inspector-budgets', 440, 800),
    alt: 'Side panel with triangles, textures, GPU memory, transfer and model budgets.',
    tool: 'Budgets',
    caption: 'per model, MB',
  },
  approvals: {
    ...screen('agents-approval'),
    alt: 'Two approval prompts in the app: Ship Moon Island, run 26, and Send Far spire to Tripo.',
    tool: 'Waiting for you',
    caption: 'two requests from an agent',
  },
  agents: {
    ...screen('settings-agents'),
    alt: 'Settings, Agents: server listening on 127.0.0.1:4791, toggles, and the activity list.',
    tool: 'Settings',
    caption: 'Agents, every call in the activity list',
  },
  toolchain: {
    ...screen('toolchain'),
    alt: 'Toolchain check: Blender, Codex CLI, ffmpeg, toktx, gltf-transform and Node, 5 of 6 found.',
    tool: 'First run',
    caption: 'the toolchain check',
  },
  providers: {
    ...screen('settings-providers'),
    alt: '3D providers: Tripo key ending 9f2c in the Keychain, Test key passed.',
    tool: 'Settings',
    caption: '3D providers, key in the Keychain',
  },
  board: {
    ...screen('board'),
    alt: 'Maquette Board with the Moon Island project open.',
    tool: 'Board',
    caption: 'Moon Island',
  },
} satisfies Record<string, Shot>;

export const NAV = [
  { href: '#features', label: 'Features' },
  { href: '#pipeline', label: 'Pipeline' },
  { href: '/docs/', label: 'Docs' },
  { href: '#download', label: 'Download' },
] as const;
