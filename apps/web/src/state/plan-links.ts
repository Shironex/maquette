/**
 * What an asset or Lab direction node shows, from the live read the core
 * makes each time (`plan_links`, crates/maquette-core/src/plan_links.rs).
 * The canvas file keeps only the id, so every name, picture and state
 * here comes from the project. Each state offers its next step.
 */
import type { Asset, AssetLink, Direction, DirectionLink, PlanNode, Stage } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';

/** The status colours of the design: muted, amber, clay, sage and red. */
export type LinkTone = 'none' | 'run' | 'pick' | 'look' | 'ok' | 'bad';

/** Where a node's next step goes. */
export type LinkAction = 'inspector' | 'inbox' | 'lab' | 'board';

export interface LinkView {
  /** The asset's or direction's name, or what stands in for it. */
  name: string;
  /** The short fact on the right of the kind line. */
  fact: string;
  status: string;
  tone: LinkTone;
  /** One quiet line under the status: numbers, a file, a count. */
  detail: string | null;
  /** The picture on this computer, when there is one. */
  picture: string | null;
  action: { to: LinkAction; label: string };
  /** The node's asset or direction is not in the project. */
  gone: boolean;
  /** What to do about a gone one, in a sentence. */
  help?: string;
}

/** The asset and direction ids a canvas links, each once, sorted. */
export function linkedIds(nodes: readonly PlanNode[]): { assets: string[]; directions: string[] } {
  const of = (kind: 'asset' | 'direction') =>
    [
      ...new Set(nodes.flatMap(n => (n.kind === kind && n[kind] ? [n[kind] as string] : []))),
    ].sort();
  return { assets: of('asset'), directions: of('direction') };
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** "18.4k tris · 2.6 MB · rigged" for a model's numbers. */
function modelLine(link: AssetLink): string | null {
  const stats = link.stats;
  if (!stats) return null;
  const parts = [`${formatK(stats.triangles)} tris`];
  if (stats.bytes) parts.push(formatBytes(stats.bytes));
  if (stats.joints > 0) parts.push('rigged');
  return parts.join(' · ');
}

/** An asset node before the first read answers. */
const reading = (what: string): LinkView => ({
  name: `Reading the ${what}...`,
  fact: '',
  status: 'Reading...',
  tone: 'none',
  detail: null,
  picture: null,
  action: { to: 'board', label: 'Open Board' },
  gone: false,
});

/** What an asset node shows for `link`, the read of its asset. */
export function assetView(link: AssetLink | undefined): LinkView {
  if (!link) return reading('asset');
  const base = {
    name: link.name ?? 'Asset not found',
    fact: link.kind ?? '',
    picture: link.thumbnail,
    gone: false,
  };
  switch (link.state) {
    case 'ready':
      return {
        ...base,
        status: `Model ready, v${link.version ?? 1}`,
        tone: 'ok',
        detail: modelLine(link),
        action: { to: 'inspector', label: 'Open in Inspector' },
      };
    case 'check':
      return {
        ...base,
        status: 'Model to check',
        tone: 'look',
        detail: `${plural(link.checks, 'model', 'models')} in the Inbox`,
        action: { to: 'inbox', label: 'Open Inbox' },
      };
    case 'no-model':
      return {
        ...base,
        status: 'No model yet',
        tone: 'none',
        detail: 'Make its concepts in the Lab',
        action: { to: 'lab', label: 'Open Lab' },
      };
    case 'file-missing':
      return {
        ...base,
        status: 'File missing',
        tone: 'bad',
        detail: link.file,
        action: { to: 'board', label: 'Open Board' },
      };
    case 'gone':
      return {
        ...base,
        status: 'Not on the Board',
        tone: 'none',
        detail: link.id,
        action: { to: 'board', label: 'Open Board' },
        gone: true,
        help: 'Restore it from Removed on the Board, or delete this node.',
      };
  }
}

/** What a Lab direction node shows for `link`, the read of its direction. */
export function directionView(link: DirectionLink | undefined): LinkView {
  if (!link) return reading('direction');
  const base = {
    name: link.name ?? 'Direction not found',
    fact: link.state === 'gone' ? '' : plural(link.images, 'image', 'images'),
    picture: link.thumbnail,
    action: { to: 'lab' as const, label: 'Open in Lab' },
    gone: false,
  };
  switch (link.state) {
    case 'running':
      return {
        ...base,
        status: 'Lab round running',
        tone: 'run',
        detail: `${link.done} of ${plural(link.done + link.pending, 'image', 'images')}`,
      };
    case 'picked':
      return {
        ...base,
        status: 'Picked',
        tone: 'pick',
        detail: `${link.picked} of ${plural(link.images, 'image', 'images')} picked`,
      };
    case 'pick':
      return {
        ...base,
        status: 'Pick images',
        tone: 'pick',
        detail: `${plural(link.images, 'image', 'images')} ready in the Lab`,
      };
    case 'empty':
      return { ...base, status: 'No images yet', tone: 'none', detail: null };
    case 'gone':
      return {
        ...base,
        status: 'Not in the Lab',
        tone: 'none',
        detail: link.id,
        gone: true,
        help: 'It was removed from the Lab. Delete this node, or link another direction.',
      };
  }
}

/** True while a linked direction has images on their way, so the canvas reads again soon. */
export function anyRunning(directions: Readonly<Record<string, DirectionLink>>): boolean {
  return Object.values(directions).some(d => d.state === 'running');
}

/** One thing the Asset picker offers: an asset of the Board or a Lab direction. */
export interface PickRow {
  key: string;
  kind: 'asset' | 'direction';
  id: string;
  name: string;
  meta: string;
  /** Relative to the project. */
  picture: string | null;
  /** Already on this canvas. */
  linked: boolean;
}

/** What the Asset picker hands the canvas: a kind and an id, nothing else. */
export type LinkPick = Pick<PickRow, 'kind' | 'id'>;

export interface PickSource {
  assets: readonly Asset[];
  directions: readonly Direction[];
  stageLabels: Readonly<Record<Stage, string>>;
}

/** The Board's assets and the Lab's directions whose name holds `query`, ignoring case. */
export function pickRows(
  source: PickSource,
  query: string,
  linked: { assets: readonly string[]; directions: readonly string[] }
): PickRow[] {
  const q = query.trim().toLowerCase();
  const assets: PickRow[] = source.assets.map(a => ({
    key: `asset:${a.id}`,
    kind: 'asset',
    id: a.id,
    name: a.name,
    meta: `${a.kind} · ${source.stageLabels[a.stage]}`,
    picture: a.thumbnail,
    linked: linked.assets.includes(a.id),
  }));
  const directions: PickRow[] = source.directions.map(d => {
    const cover = d.images.find(i => i.picked) ?? d.images.at(-1) ?? null;
    return {
      key: `direction:${d.id}`,
      kind: 'direction',
      id: d.id,
      name: d.name,
      meta: plural(d.images.length, 'image', 'images'),
      picture: cover?.file ?? null,
      linked: linked.directions.includes(d.id),
    };
  });
  return [...assets, ...directions].filter(r => !q || r.name.toLowerCase().includes(q));
}
