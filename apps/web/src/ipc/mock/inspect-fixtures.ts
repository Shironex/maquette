/**
 * The Inspector's mock data: Frostveil's three bundled models (see
 * `assets/mock/models`) laid out around the veil gate, compared as two builds
 * of the scene. v6 had one crystal cluster; v7 adds two more and Elsheria.
 * The numbers are what the files measure.
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
import { clock } from '@/lib/clock';

/** The files the mock scene serves, by the bundled model that previews them. */
export const INSPECT_FILES: Record<string, string> = {
  'web/dist/veil_gate.glb': 'veil_gate.glb',
  'web/dist/ice_crystals.glb': 'ice_crystals.glb',
  'web/dist/elsheria.glb': 'elsheria.glb',
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
  /** The material names inside the bundled file. */
  materialNames: string[];
  textures: { name: string; use: string; px: number; bytes: number; uastc?: boolean }[];
}

const MEASURED: Measured[] = [
  {
    stem: 'veil_gate',
    bytes: 412_928,
    triangles: 12_986,
    vertices: 10_213,
    size: [5.86, 6.5, 4.64],
    materials: 1,
    joints: 0,
    animations: [],
    materialNames: ['veil_gate_web'],
    textures: [
      { name: 'veil_gate_color', use: 'base color', px: 1024, bytes: 190_169 },
      { name: 'veil_gate_rm', use: 'metal and roughness', px: 512, bytes: 38_905 },
      { name: 'veil_gate_normal', use: 'normal', px: 512, bytes: 22_546 },
    ],
  },
  {
    stem: 'ice_crystals',
    bytes: 249_800,
    triangles: 7_388,
    vertices: 6_145,
    size: [0.98, 1.3, 0.82],
    materials: 1,
    joints: 0,
    animations: [],
    materialNames: ['ice_crystals_web'],
    textures: [
      { name: 'ice_crystals_color', use: 'base color', px: 512, bytes: 69_203 },
      { name: 'ice_crystals_rm', use: 'metal and roughness', px: 256, bytes: 55_496 },
      { name: 'ice_crystals_normal', use: 'normal', px: 256, bytes: 27_055 },
    ],
  },
  {
    stem: 'elsheria',
    bytes: 1_066_688,
    triangles: 34_701,
    vertices: 40_758,
    size: [0.94, 1.7, 0.44],
    materials: 1,
    joints: 119,
    animations: ['draw_sword_1', 'spellblade_idle'],
    materialNames: ['Elsheria'],
    textures: [{ name: 'Elsheria', use: 'base color', px: 1024, bytes: 224_548 }],
  },
];

/** The same estimate the Rust side makes: half a byte per pixel for ETC1S, one for UASTC, plus mips. */
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
  { name: 'Arrival', position: [9.8, 2.6, 15], target: [1.4, 2.2, 2.8], fov: 36, poster: null },
  { name: 'Veil gate', position: [-9, 3.5, 12], target: [0, 4, 0], fov: 40, poster: null },
  { name: 'Crystals', position: [-2.4, 1.1, 9.8], target: [-2.0, 0.5, 3.6], fov: 30, poster: null },
  { name: 'Keystone', position: [3.4, 1.4, 5.6], target: [0, 5.6, -0.6], fov: 58, poster: null },
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
      codec: t.uastc ? 'KTX2 UASTC' : 'KTX2 ETC1S',
      gpuBytes: ((t.px * t.px) / (t.uastc ? 1 : 2)) * (4 / 3),
    }));
    const row = presets.find(p => p.model === m.stem);
    const asset = m.stem === 'elsheria' ? 'elsheria' : m.stem === 'veil_gate' ? 'veil-gate' : null;
    const mapCodecs = row?.mapCodecs ?? null;
    const codec = row?.codec ?? defaults.codec;
    const codecs = {
      color: mapCodecs?.color ?? codec,
      // A locked row without map codecs keeps its codec for normal maps, as the core does.
      normal: mapCodecs?.normal ?? (row?.lock && !mapCodecs ? codec : 'uastc'),
      other: mapCodecs?.other ?? codec,
    } as const;
    return {
      id: `shipped/${m.stem}`,
      model: m.stem,
      label: label(m.stem, presets),
      source: 'shipped',
      file: `${projectDir}/web/dist/${m.stem}.glb`,
      path: `web/dist/${m.stem}.glb`,
      asset,
      version: null,
      modifiedAt: new Date(clock.nowMs() - 2 * 60_000).toISOString(),
      bytes: m.bytes,
      triangles: m.triangles,
      vertices: m.vertices,
      size: m.size,
      materials: m.materials,
      joints: m.joints,
      animations: m.animations,
      materialNames: m.materialNames,
      tweakAsset: asset,
      textures,
      gpuBytes: gpu(m, textures),
      meshopt: true,
      ktx2: true,
      preset: row
        ? {
            ...row,
            dataTextureSize: row.dataTextureSize ?? null,
            mapCodecs,
            codecs,
            lock: row.lock,
            hasRow: true,
          }
        : {
            ...defaults,
            dataTextureSize: null,
            mapCodecs: null,
            codecs,
            lock: null,
            hasRow: false,
          },
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
    members: [at('veil_gate', [0, 0, 0]), at('ice_crystals', [-3.6, 0, 2.4])],
  };
  const v7: CompareSide = {
    id: 'scene/v7',
    label: 'v7',
    note: 'current',
    members: [
      at('veil_gate', [0, 0, 0]),
      at('ice_crystals', [-3.6, 0, 2.4]),
      at('ice_crystals', [-0.9, 0, 4.9]),
      at('ice_crystals', [5.2, 0, 1.6]),
      at('elsheria', [3.0, 0, 4.6]),
    ],
  };
  // The character has its own budget; the other two share what is left.
  const explicit: Record<string, number> = { elsheria: 1.0 };
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
    limits: { sceneMb, trisPerModel, firstViewMb: 3 },
    scene: {
      bytes: MEASURED.reduce((s, m) => s + m.bytes, 0),
      triangles: MEASURED.reduce((s, m) => s + m.triangles, 0),
      models: MEASURED.length,
    },
  };
}
