/**
 * The page's words, kept apart from layout so the tour panels, the timeline
 * and the pipeline section stay in step. Plain and short; no dashes.
 */
import tauri from '../../../desktop/src-tauri/tauri.conf.json';
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
    short: 'A brief, refs and the style board become Codex CLI image jobs.',
    title: 'Concept',
    body: 'Write a brief, drop in up to 8 reference images and pick how many concepts you want. Each image is one Codex CLI job, four at a time. Star the good ones, pick the keepers, send picks to Refs.',
    note: 'A style board per project (a line about the look, a palette and up to four images) goes with every job, so the concepts stay in one style.',
    chips: ['codex exec', 'style board', '1 to 12 images'],
    shots: [
      {
        ...screen('lab'),
        alt: 'The Concept lab: a brief for a frozen lake arena, the project style board, and six generated concepts, one picked.',
        tool: 'Lab',
        caption: 'Frozen lake arena, 6 concepts, 1 picked',
      },
    ],
  },
  {
    id: 'refs',
    short: 'Picks land in refs/, tagged front, left, back or right.',
    title: 'Refs',
    body: "Picked images are copied into the project's refs folder and the asset moves to Refs on the Board. Tag each ref as the front, left, back or right view. Make the other views draws the missing ones from the front, and Clean background clears a busy one.",
    note: 'Before a send, a ref that is small, cropped, busy, the only view, or looks different from the front gets a warning with its fix. Warnings never block the send.',
    chips: ['refs/', 'front, left, back, right', 'Clean background'],
    shots: [
      {
        ...screen('refs-views', 560, 1000),
        alt: 'Refs for a statue: views in send order, Front and Back tagged, Left and Right empty, Make the other views, and one ref kept as Other.',
        tool: 'Refs by view',
        caption: '2 of 4 views, front first in every send',
      },
      {
        ...screen('ref-warnings', 520, 1000),
        alt: 'Send to 3D for the same statue: three ref warnings, a cropped front, a small back and a back that does not match, each with its fix.',
        tool: 'Send to 3D',
        caption: '3 ref warnings, none of them blocks',
      },
    ],
  },
  {
    id: 'model',
    short: 'Tripo, Meshy, or by hand with a settings card and no credits.',
    title: '3D model',
    body: 'Send refs to Tripo or Meshy from your preset, or make the model yourself in their web app. By hand spends nothing: Maquette writes a folder with the images and a settings card, then watches your Downloads for the GLB.',
    note: 'The Sends button in the Board header lists every send and the By hand checklist. A download named for its send goes to the inbox by itself. One whose name is only close waits for you to confirm, and any other can be attached to the asset it belongs to.',
    chips: ['Tripo', 'Meshy', 'By hand', 'Downloads watch'],
    shots: [
      {
        ...screen('send-tripo', 520, 900),
        alt: 'Send to 3D panel with Tripo selected, the project refs, one ref warning and the prop preset.',
        tool: 'Send to 3D',
        caption: 'Tripo, key from the keychain',
      },
      {
        ...screen('by-hand', 520, 900),
        alt: 'By hand card waiting for the model: open Tripo, upload the images, and every Tripo setting to copy.',
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
        alt: 'Pipeline runner: six steps, two cached, Export GLB running at 73 percent, with the baked light settings below.',
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
        ...screen('pipeline-compress', 1184, 367),
        alt: 'The live log next to the compression presets: texture size, codec and meshopt per model, with raw and shipped sizes.',
        tool: 'Pipeline',
        caption: '4 models, 14.3 MB to 3.5 MB',
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
        alt: 'A finished run: every step done or cached, Ship included, with the compression presets below.',
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
    caption: '4 models waiting, 1 failure and 2 warnings on a character',
  },
  parts: {
    ...screen('parts'),
    alt: 'Inspector, Parts tab: a lantern split into five coloured parts, two selected, with Merge, Hide and Assemble a prop.',
    tool: 'Parts',
    caption: '5 parts, split in Blender',
  },
  animations: {
    ...screen('animations'),
    alt: 'Animations library: 16 clips in four packs, tagged combat, dodge, magic and locomotion, each with its length and root motion.',
    tool: 'Animations',
    caption: '16 clips, 933 KB, skeleton only',
  },
  retarget: {
    ...screen('retarget', 1220, 720),
    alt: 'Review the retarget: four clips, one failed, sword run with the feet pinned and the slide down from 4.2 cm to 0.3 cm, source skeleton next to the model.',
    tool: 'Use on model',
    caption: 'the review before the bake, feet pinned',
  },
  avatar: {
    ...screen('avatar'),
    alt: 'Avatar view: Approve the new rest pose? over before and after sheets of a character, arms lifted to a T-pose, with the measures before and after.',
    tool: 'Avatar',
    caption: 'the rest pose waiting for a yes, VRM 1.0',
  },
  compare: {
    ...screen('inspector-compare'),
    alt: 'Scene comparison of v6 and v7 with a split slider, five camera stops below, and the selected model with its compression on the right.',
    tool: 'Compare',
    caption: 'v6 baked against v7, camera stops from stops.json',
  },
  budgets: {
    ...screen('inspector-budgets', 340, 422),
    alt: 'Side panel: a library update for the model, its textures, and model budgets with one model over its share.',
    tool: 'Budgets',
    caption: 'per model, MB',
  },
  approvals: {
    ...screen('agents-approval'),
    alt: 'Two approval prompts in the app: Ship Winter Spire, run 26, and Send Rune obelisk to Tripo.',
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
    alt: '3D providers: Tripo key ending 9f2c in the Keychain, Test key passed, 1,240 credits left.',
    tool: 'Settings',
    caption: '3D providers, key in the Keychain',
  },
  caseRoom: {
    ...screen('case-room', 1572, 756),
    alt: 'The room at the first camera stop before (left) and after (right) the ship: desk, window, shelf and a seated mascot look the same.',
    tool: 'Before and after',
    caption: 'the first camera stop, before (left) and after (right)',
  },
  caseRadio: {
    ...screen('case-radio', 1576, 440),
    alt: 'The radio up close before (left) and after (right) the ship, with the same grille, dial and knobs.',
    tool: 'Before and after',
    caption: 'the radio up close: 130 KB before, 135 KB after',
  },
  board: {
    ...screen('board'),
    alt: 'Maquette Board with a sample project open: nine assets across six columns, Concept to Ship.',
    tool: 'Board',
    caption: 'a sample project, 9 assets',
  },
} satisfies Record<string, Shot>;

/** The README's "Real projects" numbers: raw source models against what the page downloads. */
export const PROOF = {
  rows: [
    {
      scene: 'A listening corner for a website, 10 props',
      source: '27.4 MB',
      shipped: '0.89 MB',
      ratio: 'about 31x',
    },
    { scene: 'An island scene', source: '45 MB', shipped: '6.3 MB', ratio: 'about 7x' },
  ],
  armchair: {
    before: { size: '3.56 MB', triangles: '19k' },
    after: { size: '121 KB', triangles: '4k' },
  },
} as const;

/**
 * The published release the Download section links to. A new release is this
 * object: the date and the two byte sizes (the version comes from package.json). The URLs follow
 * from them; `latest/download/<file>` would break, as the file name carries the version.
 */
const REPO_URL = 'https://github.com/Shironex/maquette';
/** Read from tauri.conf.json, which bump-version.mjs keeps in step, so the links cannot lag the app. */
const VERSION: string = tauri.version;
const DOWNLOAD_BASE = `${REPO_URL}/releases/download/v${VERSION}`;

/** Bytes as megabytes with one decimal ("13.9 MB"). */
export function formatMegabytes(bytes: number): string {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

export const release = {
  version: VERSION,
  date: '29 Oct 2026',
  notesUrl: `${REPO_URL}/releases/tag/v${VERSION}`,
  allUrl: `${REPO_URL}/releases`,
  mac: {
    file: `Maquette_${VERSION}_aarch64.dmg`,
    bytes: 16_422_239,
    note: 'Apple silicon',
    url: `${DOWNLOAD_BASE}/Maquette_${VERSION}_aarch64.dmg`,
  },
  windows: {
    file: `Maquette_${VERSION}_x64-setup.exe`,
    bytes: 12_564_355,
    note: 'x64',
    url: `${DOWNLOAD_BASE}/Maquette_${VERSION}_x64-setup.exe`,
  },
} as const;

/**
 * The Mac button's label, and the short one where space is tight (the header).
 * A no-break space keeps "Apple silicon" on one line when a phone wraps the label.
 */
const MAC_NOTE = release.mac.note.replace(' ', '\u00a0');
export const MAC_LABEL = `Download for Mac (${MAC_NOTE})`;
export const MAC_SHORT = `Mac (${MAC_NOTE})`;

export const NAV = [
  { href: '#features', label: 'Features' },
  { href: '#pipeline', label: 'Pipeline' },
  { href: '/docs/', label: 'Docs' },
  { href: '#download', label: 'Download' },
] as const;
