/**
 * The numbers the Page view compares: the scene the site holds now (the
 * current ship) against this run's build, model by model, and against what
 * a device may spend. Draw calls here are an estimate from the files, one
 * per material of each model; a page that reports its own replaces it.
 */
import type { InspectModel, ModelSource } from '@/ipc/client';
import { formatBytes, formatCount } from '@/lib/format';
import type { DeviceId } from './page';

export interface SceneNumbers {
  models: number;
  bytes: number;
  triangles: number;
  gpuBytes: number;
  /** One draw per material of each model, before the page batches anything. */
  drawCalls: number;
}

/** The totals of the models from one source: `site` is the current ship, `shipped` this run. */
export function sceneNumbers(
  models: readonly InspectModel[],
  source: ModelSource
): SceneNumbers | null {
  const from = models.filter(m => m.source === source && !m.error);
  if (from.length === 0) return null;
  return {
    models: from.length,
    bytes: from.reduce((s, m) => s + (m.bytes ?? 0), 0),
    triangles: from.reduce((s, m) => s + m.triangles, 0),
    gpuBytes: from.reduce((s, m) => s + (m.gpuBytes ?? 0), 0),
    drawCalls: from.reduce((s, m) => s + Math.max(1, m.materials), 0),
  };
}

export interface CompareRow {
  label: string;
  ship: string;
  run: string;
  change: string;
  /** True when this run is better, false when worse, null when even or unknown. */
  better: boolean | null;
}

const mb = (bytes: number) => `${(bytes / 1e6).toFixed(1)} MB`;

function delta(
  a: number | undefined,
  b: number | undefined,
  show: (n: number) => string
): Pick<CompareRow, 'change' | 'better'> {
  if (a === undefined || b === undefined) return { change: '', better: null };
  const d = b - a;
  if (show(Math.abs(d)) === show(0)) return { change: '0', better: null };
  return { change: `${d > 0 ? '+' : '-'}${show(Math.abs(d))}`, better: d < 0 };
}

/** The ship table: the site's models against this run's, lower is better. */
export function compareRows(ship: SceneNumbers | null, run: SceneNumbers | null): CompareRow[] {
  const rows: [string, (n: SceneNumbers) => number, (n: number) => string][] = [
    ['Scene size', n => n.bytes, mb],
    ['Triangles', n => n.triangles, n => formatCount(Math.round(n))],
    ['GPU memory', n => n.gpuBytes, n => `${Math.round(n / 1e6)} MB`],
    ['Draw calls', n => n.drawCalls, n => formatCount(Math.round(n))],
  ];
  return rows.map(([label, pick, show]) => {
    const a = ship ? pick(ship) : undefined;
    const b = run ? pick(run) : undefined;
    return {
      label,
      ship: a === undefined ? 'none' : show(a),
      run: b === undefined ? 'none' : show(b),
      ...delta(a, b, show),
    };
  });
}

export interface ModelChange {
  title: string;
  detail: string;
}

/** What changed per model between the ship and this run, the biggest first. */
export function modelChanges(models: readonly InspectModel[]): ModelChange[] {
  const ship = new Map(models.filter(m => m.source === 'site').map(m => [m.model, m]));
  const run = models.filter(m => m.source === 'shipped');
  const out: { weight: number; change: ModelChange }[] = [];
  for (const m of run) {
    const before = ship.get(m.model);
    const bytes = m.bytes ?? 0;
    if (!before) {
      out.push({
        weight: bytes,
        change: { title: `${m.label} is new`, detail: `Adds ${formatBytes(bytes)} to the scene` },
      });
      continue;
    }
    const d = bytes - (before.bytes ?? 0);
    const tris = m.triangles - before.triangles;
    if (Math.abs(d) < 1000 && tris === 0) continue;
    const size = d < 0 ? `${formatBytes(-d)} lighter` : `${formatBytes(d)} heavier`;
    const shape =
      tris === 0 ? '' : `, ${formatCount(Math.abs(tris))} triangles ${tris < 0 ? 'fewer' : 'more'}`;
    out.push({ weight: Math.abs(d), change: { title: m.label, detail: `${size}${shape}` } });
  }
  const runIds = new Set(run.map(m => m.model));
  for (const [id, m] of ship) {
    if (runIds.has(id)) continue;
    out.push({
      weight: m.bytes ?? 0,
      change: { title: `${m.label} leaves`, detail: `Saves ${formatBytes(m.bytes ?? 0)}` },
    });
  }
  return out.sort((a, b) => b.weight - a.weight).map(o => o.change);
}

// What each device may spend.

export interface DeviceBudget {
  device: DeviceId;
  label: string;
  /** p95 frame time, ms. */
  frameMs: number;
  fps: number;
  firstViewMb: number;
  gpuMb: number;
  drawCalls: number;
}

export const DEVICE_BUDGETS: Record<DeviceId, DeviceBudget> = {
  phone: {
    device: 'phone',
    label: 'Phone, mid range',
    frameMs: 33,
    fps: 30,
    firstViewMb: 1.5,
    gpuMb: 256,
    drawCalls: 120,
  },
  laptop: {
    device: 'laptop',
    label: 'Laptop, integrated GPU',
    frameMs: 16.7,
    fps: 60,
    firstViewMb: 3,
    gpuMb: 512,
    drawCalls: 200,
  },
  desktop: {
    device: 'desktop',
    label: 'Desktop',
    frameMs: 16.7,
    fps: 60,
    firstViewMb: 3,
    gpuMb: 1024,
    drawCalls: 300,
  },
};

export interface Verdict {
  label: string;
  value: string;
  budget: string;
  over: boolean;
}

/** This run's numbers against what the device may spend. */
export function verdict(
  budget: DeviceBudget,
  run: SceneNumbers | null,
  firstViewBytes: number | null,
  p95Ms: number | null
): Verdict[] {
  const rows: Verdict[] = [];
  if (p95Ms !== null) {
    rows.push({
      label: 'Frame time, p95',
      value: `${p95Ms.toFixed(1)} ms`,
      budget: `${budget.frameMs} ms`,
      over: p95Ms > budget.frameMs,
    });
  }
  if (firstViewBytes !== null) {
    rows.push({
      label: 'First view',
      value: mb(firstViewBytes),
      budget: `${budget.firstViewMb.toFixed(1)} MB`,
      over: firstViewBytes / 1e6 > budget.firstViewMb,
    });
  }
  if (run) {
    rows.push(
      {
        label: 'GPU memory',
        value: `${Math.round(run.gpuBytes / 1e6)} MB`,
        budget: `${budget.gpuMb} MB`,
        over: run.gpuBytes / 1e6 > budget.gpuMb,
      },
      {
        label: 'Draw calls',
        value: formatCount(run.drawCalls),
        budget: formatCount(budget.drawCalls),
        over: run.drawCalls > budget.drawCalls,
      }
    );
  }
  return rows;
}
