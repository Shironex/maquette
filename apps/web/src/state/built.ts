/**
 * Built assets on the Board: models a Blender script in the project makes,
 * rebuilt headless when the script or its helpers change. These helpers say
 * how a card shows one and what a build did.
 */
import type { BuildOutcome, BuiltAsset, BuiltState } from '@/ipc/client';
import type { IconName, Status } from '@maquette/ui';

/** The file name of a path, for example "laptop.py". */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** "newtons_cradle.py" becomes "Newtons cradle", as the app names a new asset. */
export function nameFromScript(path: string): string {
  const stem = fileName(path).replace(/\.[^.]+$/, '');
  const words = stem.split(/[_\-\s.]+/).filter(Boolean);
  const text = words.join(' ').toLowerCase();
  return text ? text.charAt(0).toUpperCase() + text.slice(1) : '';
}

/** What a card's status chip says for a built asset, when it says more than the asset's own status. */
export interface BuiltChip {
  state: Status;
  label: string;
  icon?: IconName;
}

const CHIP: Partial<Record<BuiltState, BuiltChip>> = {
  new: { state: 'queued', label: 'Not built yet', icon: 'wrench' },
  stale: { state: 'review', label: 'Script changed', icon: 'wrench' },
  failed: { state: 'failed', label: 'Build failed' },
  missing: { state: 'failed', label: 'File missing', icon: 'warning' },
  building: { state: 'running', label: 'Building' },
};

export function builtChip(state: BuiltState): BuiltChip | undefined {
  return CHIP[state];
}

/** Whether the card offers Rebuild as its own button, not only in the menu. */
export function wantsRebuild(state: BuiltState): boolean {
  return state === 'new' || state === 'stale' || state === 'failed';
}

/** The line under a built card's name while it has no model yet. */
export function builtMeta(item: BuiltAsset): string {
  return `Built from ${fileName(item.source.script)}`;
}

function count(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

/** The notice after a Rebuild. */
export function outcomeNotice(name: string, outcome: BuildOutcome) {
  const record = outcome.record;
  switch (outcome.result) {
    case 'built':
      return {
        status: 'done' as const,
        title: `Built ${name} v${outcome.version?.version ?? record?.version ?? ''}`,
        detail: record?.warnings.length
          ? `Check: ${record.warnings.join(', ')}.`
          : `${count(record?.objects.length ?? 0, 'object')}, ${count(record?.materials.length ?? 0, 'material')}.`,
      };
    case 'unchanged':
      return {
        status: 'info' as const,
        title: `${name} built the same model`,
        detail: 'Blender made the newest version again byte for byte, so no version was added.',
      };
    case 'up-to-date':
      return {
        status: 'info' as const,
        title: `${name} is up to date`,
        detail: 'The script and helpers match the last build.',
      };
    case 'failed':
      return {
        status: 'failed' as const,
        title: `${name} did not build`,
        detail: record?.error ?? outcome.log.at(-1) ?? 'Blender stopped without saying why.',
      };
  }
}
