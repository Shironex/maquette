/**
 * The rules of adding a whole project to the library: which models are
 * asked about, what each choice does, the tags every model gets and the
 * name a kept clash takes.
 */
import type { AddManyPlan, AssetKind, PlannedAdd } from '@/ipc/client';
import { cleanTag } from './library-selection';

/** A clash with a model in the library: join it as a version, keep both, or leave it out. */
export type ClashChoice = 'version' | 'rename' | 'skip';

/** The collection picker's value for a new collection. */
export const NEW_COLLECTION = '__new__';

/** At or above this look a clash is offered as a version first. */
export const SAME_LOOK = 0.9;

export interface AddManyOptions {
  /** Put the project tag on every model. */
  projectTag: boolean;
  tag: string;
  /** A collection id, `NEW_COLLECTION`, or '' for none. */
  collection: string;
  newCollection: string;
  /** Comma separated tags per kind. */
  kindTags: Record<AssetKind, string>;
}

export const KIND_GROUP: Record<AssetKind, string> = {
  prop: 'Props',
  character: 'Characters',
  scene: 'Scenes',
};

export const isClash = (p: PlannedAdd) => p.state === 'name-clash' || p.state === 'looks-like';

/** What a clash starts on: a version when it looks the same, else keep both. */
export function defaultChoice(p: PlannedAdd): ClashChoice | null {
  if (!isClash(p)) return null;
  return (p.looks ?? 0) >= SAME_LOOK ? 'version' : 'rename';
}

/** The tags a model of `kind` gets: the project tag, then its kind's. */
export function plannedTags(kind: AssetKind, o: AddManyOptions): string[] {
  const tags = [...(o.projectTag ? [o.tag] : []), ...o.kindTags[kind].split(',')].map(cleanTag);
  return [...new Set(tags.filter(Boolean))];
}

/** "Stone well 2", or the next number not taken. */
export function renamedName(name: string, taken: readonly string[]): string {
  const used = new Set(taken.map(t => t.toLowerCase()));
  for (let n = 2; ; n++) {
    const next = `${name} ${n}`;
    if (!used.has(next.toLowerCase())) return next;
  }
}

export interface PlanSummary {
  /** Models that are copied as they are. */
  adding: PlannedAdd[];
  clashes: PlannedAdd[];
  sameFile: PlannedAdd[];
  unreadable: PlannedAdd[];
  /** The models that go in, by kind, with how many. */
  kinds: [AssetKind, number][];
}

export function summarise(plan: AddManyPlan): PlanSummary {
  const by = (s: PlannedAdd['state'][]) => plan.assets.filter(a => s.includes(a.state));
  const readable = plan.assets.filter(a => a.state !== 'unreadable' && a.state !== 'same-file');
  const kinds = (['prop', 'character', 'scene'] as const)
    .map((k): [AssetKind, number] => [k, readable.filter(a => a.kind === k).length])
    .filter(([, n]) => n > 0);
  return {
    adding: by(['new', 'next-version']),
    clashes: by(['name-clash', 'looks-like']),
    sameFile: by(['same-file']),
    unreadable: by(['unreadable']),
    kinds,
  };
}
