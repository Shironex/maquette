/** Pure helpers for the Board: columns, card numbers, the subtitle. */
import type { Status } from '@maquette/ui';
import type {
  Asset,
  AssetKind,
  PipelineView,
  ProjectManifest,
  Stage,
  WorkspaceSnapshot,
} from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { STAGES } from './pipeline';

export const KIND_LABEL: Record<AssetKind, string> = {
  prop: 'Prop',
  character: 'Character',
  scene: 'Scene',
};

/** True once any asset has a model, so a pipeline run has something to build. */
export const hasModels = (assets: readonly Asset[]) => assets.some(a => a.versions.length > 0);

/** A step the template did not bring: it may make what it needs, models or not. */
function hasOwnSteps({ def, starters }: Pick<PipelineView, 'def' | 'starters'>): boolean {
  const own = new Set(starters.map(s => s.path));
  return def.steps.some(s => (s.script ? !own.has(s.script) : s.kind === 'shell'));
}

/**
 * Why a run has nothing to do, or null: no asset has a model. An imported
 * folder keeps its own scene and scripts, so it may build without any, and
 * so may a project with script steps of its own.
 */
export function nothingToRun(
  project: ProjectManifest | null | undefined,
  pipeline?: Pick<PipelineView, 'def' | 'starters'> | null
): string | null {
  if (!project || project.template === 'imported' || hasModels(project.assets ?? [])) return null;
  if (pipeline && hasOwnSteps(pipeline)) return null;
  return 'No models yet, so a run has nothing to build';
}

/** What the Board's one bright button is. */
export type BoardPrimary = 'run' | 'add' | null;

/**
 * Run pipeline once there are models and no run is going; before that Add,
 * unless the empty Board's first run card already leads with its own button.
 */
export function boardPrimary(
  assets: readonly Asset[],
  { open, running, tips }: { open: boolean; running: boolean; tips: boolean }
): BoardPrimary {
  if (!open) return null;
  if (hasModels(assets)) return running ? null : 'run';
  return assets.length === 0 && tips ? null : 'add';
}

/** The project with one card in another column; the core's answer replaces it. */
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
