/** Turns detection reports into what the toolchain rows and summaries show. */
import type { ToolCheckStatus } from '@maquette/ui';
import type { ToolId, ToolReport } from '@/ipc/client';
import type { Platform } from '@/ipc/runtime';

export const BLENDER_DOWNLOAD = 'https://www.blender.org/download/';

/** The install command for this computer; Linux varies too much to name one. */
export function blenderInstallCommand(platform: Platform): string | null {
  if (platform === 'mac') return 'brew install --cask blender';
  if (platform === 'windows') return 'winget install BlenderFoundation.Blender';
  return null;
}

export type BlenderTrouble = 'missing' | 'broken' | 'stale-path';

/**
 * What is wrong with Blender. Detection only warns about a found Blender when
 * the saved location is gone and it fell back to another copy that works, so
 * builds go on; no copy is missing and a copy that fails is broken, warned or not.
 */
export function blenderTrouble(report: ToolReport | undefined): BlenderTrouble | null {
  if (!report) return null;
  if (report.status === 'missing') return 'missing';
  if (report.status === 'broken') return 'broken';
  return report.warning ? 'stale-path' : null;
}

/** `/Applications/Blender.app/Contents/MacOS/Blender` reads as the bundle. */
export function displayToolPath(path: string): string {
  const bundle = path.match(/^(.*?\.app)(?:[\\/]|$)/);
  return bundle?.[1] ?? path;
}

export function isUsable(report: ToolReport): boolean {
  return report.status === 'found';
}

export function rowStatus(report: ToolReport, checking = false): ToolCheckStatus {
  if (checking) return 'checking';
  if (report.status === 'found') return report.warning ? 'warning' : 'found';
  return 'missing';
}

/** The mono line under a tool. `roomy` is onboarding, `compact` is Settings. */
export function rowSummary(report: ToolReport, density: 'roomy' | 'compact' = 'roomy'): string {
  if (report.status !== 'found') {
    const where = report.path ? ` (${displayToolPath(report.path)})` : '';
    return `${report.detail ?? 'not usable'}${report.status === 'broken' ? where : ''}`;
  }
  if (report.source === 'npx') return report.detail ?? 'via npx';
  const path = report.path ? displayToolPath(report.path) : null;
  if (density === 'compact') {
    return [report.version, report.detail].filter(Boolean).join(' · ') || path || '';
  }
  const head = [report.version, path ? `at ${path}` : null].filter(Boolean).join(' ');
  return [head, report.detail].filter(Boolean).join(', ');
}

export function rowHint(report: ToolReport): string | undefined {
  if (report.warning && !report.hint) return report.warning;
  return report.hint ?? undefined;
}

export interface ToolSummary {
  found: number;
  total: number;
  /** Optional tools that are missing, plus found tools that need attention. */
  warnings: number;
  /** A required tool is not usable, so setup cannot continue. */
  blocked: boolean;
  missing: ToolId[];
}

export function summarizeTools(reports: readonly ToolReport[]): ToolSummary {
  const found = reports.filter(isUsable).length;
  const warnings = reports.filter(r => (isUsable(r) ? Boolean(r.warning) : !r.required)).length;
  return {
    found,
    total: reports.length,
    warnings,
    blocked: reports.some(r => r.required && !isUsable(r)),
    missing: reports.filter(r => !isUsable(r)).map(r => r.id),
  };
}

/** Detection order and labels, for rows shown before the first check returns. */
export const TOOL_ORDER: readonly { id: ToolId; label: string; required: boolean }[] = [
  { id: 'blender', label: 'Blender', required: true },
  { id: 'codex', label: 'Codex CLI', required: false },
  { id: 'ffmpeg', label: 'ffmpeg', required: false },
  { id: 'toktx', label: 'KTX-Software toktx', required: false },
  { id: 'gltf-transform', label: 'gltf-transform', required: false },
  { id: 'node', label: 'Node', required: false },
];

/** What a missing optional tool costs, for the Ready summary. */
export const MISSING_EFFECT: Record<ToolId, string> = {
  blender: 'nothing builds until it is installed',
  codex: 'the Lab cannot generate concepts until it is installed',
  ffmpeg: 'turntable videos wait until it is installed',
  toktx: 'textures stay PNG, so models download larger and use more GPU memory',
  'gltf-transform': 'Compress waits until it is installed',
  node: 'gltf-transform needs it through npx',
};

export const SHORT_NAME: Record<ToolId, string> = {
  blender: 'Blender',
  codex: 'Codex CLI',
  ffmpeg: 'ffmpeg',
  toktx: 'toktx',
  'gltf-transform': 'gltf-transform',
  node: 'Node',
};
