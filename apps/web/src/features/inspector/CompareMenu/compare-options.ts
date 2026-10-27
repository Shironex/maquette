import type { AssetKind, InspectView } from '@/ipc/client';
import type { IconName } from '@maquette/ui';
import type { SidePick } from '@/state/inspector';

export interface CompareOption {
  id: string;
  label: string;
  /** A quiet trailing note, such as "3 versions". */
  detail?: string;
  icon: IconName;
  /** The comparison already open: shown and scrolled to, never picked. */
  current: boolean;
  onSelect: () => void;
}

export interface CompareGroup {
  id: string;
  title: string;
  options: CompareOption[];
}

export type ChooseSides = (patch: {
  comparison?: string;
  left?: string | null;
  right?: string | null;
}) => void;

const KIND_GROUPS: { kind: AssetKind | 'other'; title: string }[] = [
  { kind: 'character', title: 'Characters' },
  { kind: 'prop', title: 'Props' },
  { kind: 'scene', title: 'Scenes' },
  { kind: 'other', title: 'Other' },
];

const ASSET_PREFIX = 'asset/';

/** Case-insensitive name order, so "armillary sphere" sits with the capitalised names. */
export function byName(a: { label: string }, b: { label: string }): number {
  return a.label.localeCompare(b.label, undefined, { sensitivity: 'base' });
}

function versionsNote(count: number): string {
  return `${count} ${count === 1 ? 'version' : 'versions'}`;
}

/**
 * What the compare menu offers, in groups: the other versions of this
 * comparison first, then the whole scene, then each asset under its kind,
 * every group sorted by name regardless of case.
 */
export function compareGroups(
  view: InspectView,
  pick: SidePick,
  kinds: ReadonlyMap<string, AssetKind>,
  choose: ChooseSides
): CompareGroup[] {
  const groups: CompareGroup[] = [];
  const others = pick.comparison.sides
    .filter(s => s.id !== pick.right.id && s.id !== pick.left?.id)
    .map(s => ({
      id: `l-${s.id}`,
      label: `Compare ${s.label} with ${pick.right.label}`,
      icon: 'arrows-left-right' as const,
      current: false,
      onSelect: () => choose({ left: s.id }),
    }));
  if (others.length)
    groups.push({ id: 'versions', title: `Versions of ${pick.comparison.label}`, options: others });

  const option = (c: InspectView['comparisons'][number]): CompareOption => ({
    id: `c-${c.id}`,
    label: c.label,
    detail: c.kind === 'scene' ? undefined : versionsNote(c.sides.length),
    icon: c.kind === 'scene' ? 'squares-four' : 'cube',
    current: c.id === pick.comparison.id,
    onSelect: () => choose({ comparison: c.id, left: null, right: null }),
  });
  const scene = view.comparisons.filter(c => c.kind === 'scene');
  if (scene.length) groups.push({ id: 'scene', title: 'Scene', options: scene.map(option) });

  const assets = view.comparisons.filter(c => c.kind !== 'scene');
  for (const { kind, title } of KIND_GROUPS) {
    const members = assets
      .filter(c => (kinds.get(c.id.slice(ASSET_PREFIX.length)) ?? 'other') === kind)
      .map(option)
      .sort(byName);
    if (members.length) groups.push({ id: `kind-${kind}`, title, options: members });
  }
  return groups;
}

/** The groups with only the options whose name has the query, empty groups dropped. */
export function filterGroups(groups: CompareGroup[], query: string): CompareGroup[] {
  const q = query.trim().toLowerCase();
  if (!q) return groups;
  return groups
    .map(g => ({ ...g, options: g.options.filter(o => o.label.toLowerCase().includes(q)) }))
    .filter(g => g.options.length > 0);
}

/** The options a key can land on, in the order shown. */
export function pickable(groups: CompareGroup[]): CompareOption[] {
  return groups.flatMap(g => g.options).filter(o => !o.current);
}

/** The DOM id of an option, so the search box can name the active one. */
export function optionDomId(listId: string, option: CompareOption): string {
  return `${listId}-${option.id}`;
}
