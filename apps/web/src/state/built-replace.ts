/**
 * A built asset in the place of another model: how the sheet words the
 * report of a replace and of its undo, and the line the Ship approval
 * shows while a swap has not shipped yet.
 */
import {
  commands,
  type ReplaceReport,
  type ShipSwap,
  type SwapAction,
  type SwapSetting,
  type SwapSide,
  type UndoReport,
} from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';

/** What the replace sheet is open for: a pair to confirm, or the asset whose replace it undoes. */
export type ReplaceTarget =
  { mode: 'replace'; asset: string | null; built: string | null } | { mode: 'undo'; asset: string };

/** How the sheet groups a replace's settings, in the order it lists them. */
export const SETTING_GROUPS: readonly { action: SwapAction; title: string; hint: string }[] = [
  {
    action: 'carried',
    title: 'Carries over',
    hint: 'Recorded for the old model and right for the new one',
  },
  {
    action: 'brought',
    title: 'Comes with the built asset',
    hint: 'Its own settings, made for this model',
  },
  {
    action: 'reset',
    title: 'Taken off',
    hint: 'Recorded for the old model only, and back when the old model is',
  },
  {
    action: 'kept',
    title: 'Stays as it is',
    hint: 'Keyed by the asset, which keeps its id',
  },
];

/** The settings of one group, in the order the report has them. */
export function settingsOf(report: ReplaceReport, action: SwapAction): readonly SwapSetting[] {
  return report.settings.filter(s => s.action === action);
}

/** "36 triangles · 2.1 KB", leaving out what is not known. */
export function modelNumbers(side: { triangles: number | null; bytes: number | null }): string {
  return [
    side.triangles === null ? null : `${formatK(side.triangles)} triangles`,
    side.bytes === null ? null : formatBytes(side.bytes),
  ]
    .filter(Boolean)
    .join(' · ');
}

function trim(value: number): string {
  return String(Number(value.toFixed(1)));
}

/** "30 × 10 × 10 cm" (width, height, depth) as the model stands, or null without bounds. */
export function standingSize(side: SwapSide): string | null {
  const { min, max } = side;
  if (!min || !max) return null;
  const extent = [0, 1, 2].map(k => (max[k] ?? 0) - (min[k] ?? 0));
  const longest = Math.max(...extent);
  const [unit, by] = longest < 0.01 ? ['mm', 1000] : longest < 1 ? ['cm', 100] : ['m', 1];
  return `${extent.map(v => trim(v * by)).join(' × ')} ${unit}`;
}

/** "scaled by 0.6" for a model its real size scales, or "not scaled". */
export function scaleWords(side: SwapSide): string {
  const scale = side.scale ?? 1;
  return Math.abs(scale - 1) < 0.005 ? 'not scaled' : `scaled by ${trim2(scale)}`;
}

function trim2(value: number): string {
  return String(Number(value.toFixed(2)));
}

/** How many lines of a replace's report ask for a look. */
export function toCheck(report: ReplaceReport): number {
  return report.warnings.length + report.unresolved.length;
}

/** The notice after a replace. */
export function replacedNotice(report: ReplaceReport) {
  const check = toCheck(report);
  return {
    status: check ? ('info' as const) : ('done' as const),
    title: `${report.builtName} is in ${report.name}'s place`,
    detail: `v${report.new.version} is the built model${
      check ? `, with ${check} ${check === 1 ? 'thing' : 'things'} to check` : ''
    }. Bring back the old model undoes it.`,
  };
}

/** The notice after an undo. */
export function undoneNotice(report: UndoReport) {
  return {
    status: report.warnings.length ? ('info' as const) : ('done' as const),
    title: `${report.name} is back on v${report.back.version}`,
    detail: report.warnings.length
      ? report.warnings.join(' ')
      : report.restores
        ? `${report.restores} is back on the Board as a built asset.`
        : 'The model it had before is current again.',
  };
}

/**
 * What the Ship approval says about one swap: the asset, the two files
 * and the numbers of both models.
 */
export function swapLine(swap: ShipSwap): string {
  const { oldFile, newFile } = swap;
  const files =
    oldFile && newFile && oldFile !== newFile
      ? `${newFile} takes the place of ${oldFile}`
      : (newFile ?? oldFile ?? `v${swap.newVersion} takes the place of v${swap.oldVersion}`);
  const triangles =
    swap.oldTriangles === null || swap.newTriangles === null
      ? null
      : `${formatK(swap.newTriangles)} triangles, was ${formatK(swap.oldTriangles)}`;
  const bytes =
    swap.oldBytes === null || swap.newBytes === null
      ? null
      : `${formatBytes(swap.newBytes)}, was ${formatBytes(swap.oldBytes)}`;
  return [`${swap.name} is a built model now: ${files}`, triangles, bytes]
    .filter(Boolean)
    .join(' · ');
}

/** "-0.49, 0, -0.41 to 0.49, 1.3, 0.41": a side's bounds in metres, or null without them. */
export function boundsWords(side: SwapSide): string | null {
  const { min, max } = side;
  if (!min || !max) return null;
  const point = (p: readonly (number | null)[]) => p.map(v => trim2(v ?? 0)).join(', ');
  return `${point(min)} to ${point(max)}`;
}

/** The lines for the swaps of `project` that no Ship has written yet; none when they cannot be read. */
export async function swapLines(project: string): Promise<string[]> {
  try {
    return (await commands.builtSwaps(project)).map(swapLine);
  } catch {
    return [];
  }
}
