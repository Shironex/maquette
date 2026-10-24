/**
 * The Inspector's mock data: the moon island's three bundled models (see
 * `assets/mock/models`) laid out under the sakura, compared as two builds of
 * the scene. v6 had one shrub; v7 adds two more and the character. The
 * numbers are what the Rust analysis measures for those files.
 */
import type {
  CameraStopDef,
  CompareSide,
  CompressPreset,
  InspectModel,
  InspectView,
  MeshoptLevel,
  TextureCodec,
  TextureRow,
} from '../client';

/** The files the mock scene serves, by the bundled model that previews them. */
export const INSPECT_FILES: Record<string, string> = {
  'web/dist/sakura.glb': 'sakura_tree_large.glb',
  'web/dist/flower_shrubs.glb': 'flower_shrubs.glb',
  'web/dist/character.glb': 'character.glb',
};

interface Measured {
  stem: string;
  bytes: number;
  triangles: number;
  vertices: number;
  size: [number, number, number];
  materials: number;
  joints: number;
  animations: string[];
  textures: { name: string; use: string; px: number; bytes: number }[];
}

const MEASURED: Measured[] = [
  {
    stem: 'sakura',
    bytes: 330_444,
    triangles: 28_264,
    vertices: 24_289,
    size: [11.68, 7.97, 7.63],
    materials: 1,
    joints: 0,
    animations: [],
    textures: [{ name: 'sakura_tree_large_web', use: 'base color', px: 512, bytes: 60_109 }],
  },
  {
    stem: 'flower_shrubs',
    bytes: 169_808,
    triangles: 9_671,
    vertices: 9_465,
    size: [1.57, 0.67, 1.32],
    materials: 1,
    joints: 0,
    animations: [],
    textures: [{ name: 'flower_shrubs_web', use: 'base color', px: 512, bytes: 58_244 }],
  },
  {
    stem: 'character',
    bytes: 1_511_800,
    triangles: 39_613,
    vertices: 39_834,
    size: [1.36, 2.0, 0.83],
    materials: 2,
    joints: 75,
    animations: ['sitting_idle_hug', 'sitting_wave'],
    textures: [
      { name: 'Color', use: 'base color', px: 2048, bytes: 696_840 },
      { name: 'Gift boxes', use: 'base color', px: 512, bytes: 75_069 },
    ],
  },
];

/** The same estimate the Rust side makes: ETC1S at half a byte per pixel plus mips. */
function gpu(m: Measured, rows: TextureRow[]): number {
  const geometry = m.vertices * (m.joints > 0 ? 48 : 32) + m.triangles * 12;
  return rows.reduce((sum, t) => sum + (t.gpuBytes ?? 0), 0) + geometry;
}

function label(stem: string, presets: CompressPreset[]): string {
  return (
    presets.find(p => p.model === stem)?.label ??
    stem.replace(/_/g, ' ').replace(/^./, c => c.toUpperCase())
  );
}

const STOPS: CameraStopDef[] = [
  { name: 'Porch', position: [9.8, 2.6, 15], target: [1.4, 2.2, 2.8], fov: 36, poster: null },
  { name: 'Sakura', position: [-9, 3.5, 12], target: [0, 4, 0], fov: 40, poster: null },
  { name: 'Shrubs', position: [-2.4, 1.1, 9.8], target: [-2.0, 0.5, 3.6], fov: 30, poster: null },
  { name: 'Canopy', position: [3.4, 1.4, 5.6], target: [0, 5.6, -0.6], fov: 58, poster: null },
  { name: 'Pullback', position: [14, 7.5, 19], target: [0.5, 3, 1], fov: 35, poster: null },
];

export function inspectView(
  projectId: string,
  projectDir: string,
  presets: CompressPreset[],
  defaults: { textureSize: number; codec: TextureCodec; meshopt: MeshoptLevel },
  sceneMb: number,
  trisPerModel: number
): InspectView {
  const models: InspectModel[] = MEASURED.map(m => {
    const textures: TextureRow[] = m.textures.map(t => ({
      name: t.name,
      uses: [t.use],
      width: t.px,
      height: t.px,
      bytes: t.bytes,
      codec: 'KTX2 ETC1S',
      gpuBytes: ((t.px * t.px) / 2) * (4 / 3),
    }));
    const row = presets.find(p => p.model === m.stem);
    return {
      id: `shipped/${m.stem}`,
      model: m.stem,
      label: label(m.stem, presets),
      source: 'shipped',
      file: `${projectDir}/web/dist/${m.stem}.glb`,
      path: `web/dist/${m.stem}.glb`,
      asset: m.stem === 'character' ? 'character' : m.stem === 'sakura' ? 'sakura' : null,
      version: null,
      modifiedAt: new Date(Date.now() - 2 * 60_000).toISOString(),
      bytes: m.bytes,
      triangles: m.triangles,
      vertices: m.vertices,
      size: m.size,
      materials: m.materials,
      joints: m.joints,
      animations: m.animations,
      textures,
      gpuBytes: gpu(m, textures),
      meshopt: true,
      ktx2: true,
      preset: row
        ? { ...row, dataTextureSize: row.dataTextureSize ?? null, lock: row.lock, hasRow: true }
        : { ...defaults, dataTextureSize: null, lock: null, hasRow: false },
      error: null,
    };
  });
  const at = (model: string, position: [number, number, number]) => ({
    model: `shipped/${model}`,
    position,
  });
  const v6: CompareSide = {
    id: 'scene/v6',
    label: 'v6',
    note: 'baked Oct 23',
    members: [at('sakura', [0, 0, 0]), at('flower_shrubs', [-3.6, 0, 2.4])],
  };
  const v7: CompareSide = {
    id: 'scene/v7',
    label: 'v7',
    note: 'current',
    members: [
      at('sakura', [0, 0, 0]),
      at('flower_shrubs', [-3.6, 0, 2.4]),
      at('flower_shrubs', [-0.9, 0, 4.9]),
      at('flower_shrubs', [5.2, 0, 1.6]),
      at('character', [3.0, 0, 4.6]),
    ],
  };
  // The character has its own budget; the other two share what is left.
  const explicit: Record<string, number> = { character: 1.0 };
  const open = MEASURED.filter(m => explicit[m.stem] === undefined).length;
  const share = Math.round(((sceneMb - 1.0) / open) * 100) / 100;
  const budgets = MEASURED.map(m => ({
    model: m.stem,
    label: label(m.stem, presets),
    bytes: m.bytes,
    targetMb: explicit[m.stem] ?? share,
    explicit: explicit[m.stem] !== undefined,
    triangles: m.triangles,
  }));
  return {
    project: projectId,
    models,
    comparisons: [{ id: 'scene', label: 'Scene', kind: 'scene', sides: [v6, v7] }],
    stops: { source: 'project-file', file: 'stops.json', stops: STOPS },
    budgets,
    limits: { sceneMb, trisPerModel },
    scene: {
      bytes: MEASURED.reduce((s, m) => s + m.bytes, 0),
      triangles: MEASURED.reduce((s, m) => s + m.triangles, 0),
      models: MEASURED.length,
    },
  };
}
