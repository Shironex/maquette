/**
 * Data for mock mode. Neutral names and paths only, so screenshots never
 * point at a real project on someone's disk.
 */
import type { ProjectManifest, ToolReport, WorkspaceConfig } from '../client';

export const HOME = '/Users/maker';
export const DEFAULT_ROOT = '~/Maquette';
export const SITE_REPO = '~/Documents/Projects/moon-island-site';

export function defaultConfig(): WorkspaceConfig {
  return {
    version: 1,
    layout: { projects: 'projects', inbox: 'inbox', cache: 'cache' },
    tripo: {
      model: 'H3.1',
      hd: true,
      topology: 'triangle',
      texture: '2K',
      removeLighting: true,
      pbr: true,
      character: { quads: 50000, multiView: true, autoRig: true, rigPreset: 'mixamo' },
    },
    budgets: { sceneMb: 3, trisPerModel: 60000 },
    compression: { textureCodec: 'etc1s', meshopt: 'medium' },
    agents: {
      mcpEnabled: false,
      mcpPort: 4791,
      rules: { blenderHeadlessOnly: true, neverCaptureScreen: true, askBeforeShipping: true },
    },
    currentProject: null,
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
    report({ id: 'ffmpeg', label: 'ffmpeg', path: '/opt/homebrew/bin/ffmpeg', version: '7.1' }),
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

export function moonIsland(): ProjectManifest {
  return {
    version: 1,
    id: 'moon-island',
    name: 'Moon Island',
    template: 'floating-diorama',
    siteRepo: `${HOME}/Documents/Projects/moon-island-site`,
    sourceFolder: null,
    createdAt: new Date().toISOString(),
  };
}
