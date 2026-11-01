/** Pure helpers for the Board: columns, card numbers, the subtitle. */
import type { Status } from '@maquette/ui';
import type { Asset, AssetKind, ProjectManifest, Stage, WorkspaceSnapshot } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { STAGES } from './pipeline';

export const KIND_LABEL: Record<AssetKind, string> = {
  prop: 'Prop',
  character: 'Character',
  scene: 'Scene',
};

/** The project with one card in another column; the answer from disk replaces it. */
export function withStage(project: ProjectManifest, id: string, stage: Stage): ProjectManifest {
  return {
    ...project,
    assets: (project.assets ?? []).map(a => (a.id === id ? { ...a, stage } : a)),
  };
}

export function columns(assets: readonly Asset[]): Record<Stage, Asset[]> {
  const out = Object.fromEntries(STAGES.map(s => [s, [] as Asset[]])) as Record<Stage, Asset[]>;
  for (const asset of assets) out[asset.stage].push(asset);
  for (const list of Object.values(out)) {
    list.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt) || a.name.localeCompare(b.name));
  }
  return out;
}

export interface CardNumbers {
  version?: number;
  tris?: string;
  size?: string;
  status: Status;
}

export function cardNumbers(asset: Asset): CardNumbers {
  const latest = asset.versions[asset.versions.length - 1];
  const stats = latest?.stats;
  return {
    version: latest?.version,
    tris: stats ? formatK(stats.triangles) : undefined,
    size: stats?.bytes ? formatBytes(stats.bytes) : undefined,
    status: asset.status,
  };
}

/** "4 assets", "1 asset", "No assets yet". */
export function assetCount(n: number): string {
  if (n === 0) return 'No assets yet';
  return `${n} ${n === 1 ? 'asset' : 'assets'}`;
}

/** The absolute path of a file inside the current project. */
export function projectFile(workspace: WorkspaceSnapshot, relative: string): string | null {
  const project = workspace.project;
  if (!project) return null;
  const sep = workspace.root.includes('\\') ? '\\' : '/';
  return [
    workspace.root,
    workspace.config.layout.projects,
    project.id,
    ...relative.split('/'),
  ].join(sep);
}
