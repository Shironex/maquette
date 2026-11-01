/**
 * Data for mock mode. Neutral names and paths only, so screenshots never
 * point at a real project on someone's disk.
 */
import type {
  MeshyPreset,
  ProviderCatalog,
  ToolReport,
  TripoPreset,
  WorkspaceConfig,
} from '../client';

export function defaultConfig(): WorkspaceConfig {
  return {
    version: 1,
    layout: { projects: 'projects', inbox: 'inbox', cache: 'cache' },
    providers: { tripo: tripoPresets(), meshy: meshyPresets() },
    budgets: { sceneMb: 3, trisPerModel: 60000, firstViewMb: 3 },
    compression: { textureSize: '2K', textureCodec: 'etc1s', meshopt: 'medium' },
    agents: {
      mcpEnabled: false,
      mcpPort: 4791,
      rules: {
        blenderHeadlessOnly: true,
        neverCaptureScreen: true,
        askBeforeShipping: true,
        avatarRigTools: true,
      },
    },
    lab: { concurrency: 4, timeoutMinutes: 10 },
    currentProject: null,
  };
}

/** Mirrors `TripoPreset::prop` and `::character` in the Rust core. */
function tripoPresets(): WorkspaceConfig['providers']['tripo'] {
  const prop = {
    model: 'v3.1-20260211',
    faceLimit: 0,
    texture: true,
    pbr: true,
    textureQuality: 'standard',
    geometryQuality: 'standard',
    smartLowPoly: false,
    autoSize: true,
    imageAutofix: false,
    multiView: false,
    rig: false,
    rigModel: 'v3.0-20260909',
    rigSpec: 'mixamo',
    animations: [],
  } satisfies TripoPreset;
  return {
    prop,
    character: {
      ...prop,
      faceLimit: 50000,
      multiView: true,
      rig: true,
      animations: ['preset:idle', 'preset:walk'],
    },
  };
}

/** Mirrors `MeshyPreset::prop` and `::character` in the Rust core. */
function meshyPresets(): WorkspaceConfig['providers']['meshy'] {
  const prop = {
    aiModel: 'latest',
    shouldTexture: true,
    enablePbr: true,
    textureResolution: '2k',
    shouldRemesh: false,
    topology: 'triangle',
    targetPolycount: 30000,
    poseMode: 'none',
    removeLighting: true,
    multiView: false,
    rig: false,
    heightMeters: 1.7,
  } satisfies MeshyPreset;
  return {
    prop,
    character: {
      ...prop,
      shouldRemesh: true,
      targetPolycount: 50000,
      poseMode: 'a-pose',
      multiView: true,
      rig: true,
    },
  };
}

const option = (id: string, label: string, note = '') => ({ id, label, note });

/** A copy of `providers::catalog::catalog()` for mock mode. */
export function providerCatalog(): ProviderCatalog {
  return {
    tripoModels: [
      option('v3.1-20260211', 'H3.1', 'Latest high detail model'),
      option('v3.0-20250812', 'H3.0', 'Previous stable version'),
      option('v2.5-20250123', 'H2.5', 'Legacy, for older projects'),
      option('P2-20260801', 'P2 Preview', 'Game ready, 48 to 50,000 triangles'),
      option('P1-20260311', 'P1', 'Low poly, 50 to 20,000 faces'),
    ],
    tripoRigModels: [
      option('v3.0-20260909', 'Rig 3.0', 'Best for humanoids'),
      option('v2.5-20260210', 'Rig 2.5', 'Creatures and other shapes'),
      option('v1.0-20240301', 'Rig 1.0', 'Humanoids only, the most presets'),
    ],
    tripoAnimations: (
      [
        ['idle', 'Idle'],
        ['walk', 'Walk'],
        ['run', 'Run'],
        ['jump', 'Jump'],
        ['turn', 'Turn'],
        ['fall', 'Fall'],
        ['hurt', 'Hurt'],
        ['slash', 'Slash'],
        ['shoot', 'Shoot'],
        ['climb', 'Climb'],
        ['dive', 'Dive'],
      ] as const
    ).map(([id, label]) => option(`preset:${id}`, label)),
    meshyModels: [
      option('latest', 'Meshy 7.1', 'Latest standard model'),
      option('meshy-6', 'Meshy 6', 'Removes baked lighting'),
      option('meshy-6-lite', 'Meshy 6 Lite', 'Faster and cheaper'),
      option('meshy-t2.5', 'Meshy T2.5', 'Smart Topology, sharp game assets'),
      option('meshy-t2', 'Meshy T2', 'Smart Topology, separated parts'),
    ],
  };
}

function report(partial: Partial<ToolReport> & Pick<ToolReport, 'id' | 'label'>): ToolReport {
  return {
    required: false,
    status: 'found',
    source: 'path',
    path: null,
    version: null,
    detail: null,
    hint: null,
    warning: null,
    ...partial,
  };
}

export function toolReports(scenario: string): ToolReport[] {
  const blender =
    scenario === 'no-blender'
      ? report({
          id: 'blender',
          label: 'Blender',
          required: true,
          status: 'missing',
          source: null,
          detail: 'not found on PATH',
          hint: 'Install Blender from blender.org, or locate Blender.app.',
        })
      : report({
          id: 'blender',
          label: 'Blender',
          required: true,
          source: 'known-location',
          path: '/Applications/Blender.app/Contents/MacOS/Blender',
          version: '4.3.2',
          detail: 'runs headless',
        });
  return [
    blender,
    report({
      id: 'codex',
      label: 'Codex CLI',
      path: '/opt/homebrew/bin/codex',
      version: '0.46.0',
      detail: 'signed in',
    }),
    scenario === 'no-ffmpeg'
      ? report({
          id: 'ffmpeg',
          label: 'ffmpeg',
          status: 'missing',
          source: null,
          detail: 'not found on PATH',
        })
      : report({ id: 'ffmpeg', label: 'ffmpeg', path: '/opt/homebrew/bin/ffmpeg', version: '7.1' }),
    report({
      id: 'toktx',
      label: 'KTX-Software toktx',
      status: 'missing',
      source: null,
      detail: 'not found on PATH',
      hint: 'Install KTX-Software, then Re-check. Until then Compress keeps textures as PNG.',
    }),
    report({
      id: 'gltf-transform',
      label: 'gltf-transform',
      source: 'npx',
      path: '/opt/homebrew/bin/npx',
      detail: 'via npx, @gltf-transform/cli',
    }),
    report({ id: 'node', label: 'Node', path: '/opt/homebrew/bin/node', version: '22.23.0' }),
  ];
}

export {
  MOCK_PROJECT_IDS,
  elsheriaShowcase,
  mockProjects,
  frostveilAssets,
  frostveilProject,
  moreProjects,
  frostCathedral,
} from './project-fixtures';
export { DEFAULT_ROOT, HOME, SITE_REPO } from './mock-paths';
