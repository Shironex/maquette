/**
 * `?inbox=triage`: a busy inbox to go through many at a time. Besides the
 * four usual models (linked to their sends as for `?inbox=redo`) it holds
 * eight more: five whose checks pass (from Tripo, Meshy and By hand sends, a
 * download attached to an asset and a model nobody sent), one over budget,
 * and two older models a redo replaced (the asset got a newer version after
 * they arrived). Three models were discarded earlier, so Recently discarded
 * has something to restore. `?inbox=clear` is the inbox after a round: empty,
 * with the same three discarded models.
 */
import type { DiscardedModel, InboxItem, InboxLink, Provider } from '../client';
import { inboxItems, thumbsDir } from './inbox-fixtures';
import { linkForRedo } from './redo-fixtures';

const MIN = 60_000;
const HOUR = 60 * MIN;

interface Spec {
  id: string;
  name: string;
  thumb: string;
  triangles: number;
  mb: number;
  ago: number;
  link?: [project: string, asset: string, assetName: string, provider: Provider];
  aim?: [project: string, asset: string, assetName: string];
  /** An over budget model: its triangle check warns. */
  heavy?: boolean;
}

const SPECS: Spec[] = [
  {
    id: 'guardian_statues.glb',
    name: 'Guardian statues',
    thumb: 'guardian-statue.webp',
    triangles: 33_700,
    mb: 6.2,
    ago: 6 * MIN,
    link: ['frostveil', 'guardian-statues', 'Guardian statues', 'by-hand'],
  },
  {
    id: 'frozen_angel.glb',
    name: 'Frozen angel',
    thumb: 'frozen-angel.webp',
    triangles: 24_800,
    mb: 4.9,
    ago: 7 * MIN,
    link: ['frostveil', 'frozen-angel', 'Frozen angel', 'meshy'],
  },
  {
    id: 'observatory_arch_v4.glb',
    name: 'Observatory arch',
    thumb: 'observatory-arch.webp',
    triangles: 38_200,
    mb: 5.4,
    ago: 11 * MIN,
    link: ['frostveil', 'observatory-arch', 'Observatory arch', 'tripo'],
  },
  {
    id: 'lantern_post_3d_model.glb',
    name: 'Lantern post 3d model',
    thumb: 'crystal-lantern.webp',
    triangles: 12_900,
    mb: 2.1,
    ago: 12 * MIN,
    aim: ['frost-cathedral', 'crystal-lantern', 'Crystal lantern'],
  },
  {
    id: 'veil_platform_scan.glb',
    name: 'Veil platform scan',
    thumb: 'veil-platform.webp',
    triangles: 41_000,
    mb: 7.7,
    ago: 13 * MIN,
  },
  {
    id: 'crypt_brazier.glb',
    name: 'Crypt brazier',
    thumb: 'crypt-brazier.webp',
    triangles: 609_000,
    mb: 41,
    ago: 16 * MIN,
    link: ['frostveil', 'crypt-brazier', 'Crypt brazier', 'meshy'],
    heavy: true,
  },
  {
    id: 'crystal_lantern_redo.glb',
    name: 'Crystal lantern',
    thumb: 'crystal-lantern.webp',
    triangles: 19_600,
    mb: 3.4,
    ago: 50 * HOUR,
    link: ['frostveil', 'crystal-lantern', 'Crystal lantern', 'by-hand'],
  },
  {
    id: 'observatory_arch_v3.glb',
    name: 'Observatory arch',
    thumb: 'observatory-arch.webp',
    triangles: 12_300,
    mb: 2.2,
    ago: 30 * HOUR,
    link: ['frostveil', 'observatory-arch', 'Observatory arch', 'tripo'],
  },
];

/** Which bundled model previews each triage model. */
export const TRIAGE_PREVIEW: Record<string, string> = {
  'guardian_statues.glb': 'sigil_gate.glb',
  'frozen_angel.glb': 'clay_figure.glb',
  'observatory_arch_v4.glb': 'sigil_gate_frame.glb',
  'lantern_post_3d_model.glb': 'frost_lantern.glb',
  'veil_platform_scan.glb': 'sigil_gate_steps.glb',
  'crypt_brazier.glb': 'rune_chest.glb',
  'crystal_lantern_redo.glb': 'frost_lantern_tripo.glb',
  'observatory_arch_v3.glb': 'sigil_gate_frame.glb',
};

function link(spec: Spec): InboxLink | null {
  if (!spec.link) return null;
  const [project, asset, assetName, provider] = spec.link;
  return { job: `${asset}-1`, project, asset, assetName, provider };
}

/** One triage model, shaped from the veil gate (a prop whose checks pass). */
function triageItem(base: InboxItem, dir: string, now: number, spec: Spec): InboxItem {
  const k = (spec.triangles / 1000).toFixed(1);
  const checks = base.checks.map(check =>
    check.id !== 'triangles'
      ? check
      : spec.heavy
        ? {
            ...check,
            status: 'warning' as const,
            title: `${k}k triangles`,
            detail:
              'Over the 60k per model budget. Decimate it here, or raise the budget in Settings.',
            fixes: [{ op: 'decimate' as const, triangles: 60_000 }],
          }
        : { ...check, title: `${k}k triangles` }
  );
  return {
    ...base,
    id: spec.id,
    name: spec.name,
    file: `${dir}/${spec.id}`,
    bytes: spec.mb * 1e6,
    modifiedMs: now - spec.ago,
    kindGuessed: !spec.link,
    kindFromSend: Boolean(spec.link),
    report: base.report
      ? { ...base.report, triangles: spec.triangles, bytes: spec.mb * 1e6 }
      : null,
    checks,
    worst: spec.heavy ? 'warning' : 'pass',
    thumbnail: `${thumbsDir(dir)}/${spec.thumb}`,
    link: link(spec),
    aim: spec.aim ? { project: spec.aim[0], asset: spec.aim[1], assetName: spec.aim[2] } : null,
  };
}

/** How long the core keeps a discarded model. */
export const KEEP_MS = 30 * 24 * HOUR;

/** The discarded models: their notes, and the model each puts back. */
export interface DiscardedSeed {
  model: DiscardedModel;
  item: InboxItem;
}

function discardedSeeds(dir: string, now: number, base: InboxItem): DiscardedSeed[] {
  const rows: [Spec, number, number | null][] = [
    [{ ...SPECS[6]!, id: 'crystal_lantern_v1.glb', ago: 3 * 24 * HOUR }, 14 * MIN, 2],
    [{ ...SPECS[7]!, id: 'observatory_arch_v2.glb', ago: 4 * 24 * HOUR }, 22 * MIN, 3],
    [{ ...SPECS[5]!, id: 'crypt_brazier_dense.glb', ago: 5 * HOUR }, 3 * HOUR, null],
  ];
  return rows.map(([spec, ago, replacedBy]) => {
    const item = triageItem(base, dir, now, spec);
    return {
      item,
      model: {
        name: spec.id,
        id: spec.id,
        file: `${dir}/.discarded/${spec.id}`,
        bytes: item.bytes,
        discardedMs: now - ago,
        expiresMs: now - ago + KEEP_MS,
        assetName: spec.link?.[2] ?? null,
        replacedBy,
      },
    };
  });
}

/** The inbox a mock workspace opens with, for `?inbox=`. */
export function seedInbox(state: string | null, dir: string, now: number): InboxItem[] {
  const listed = inboxItems(dir, now);
  if (state === 'clear') return [];
  if (state === 'redo') return linkForRedo(listed);
  if (state === 'retexture') {
    const base = listed.find(i => i.id === 'veil_gate.glb')!;
    return [repaintedFern(base, dir, now), splitWaitingToPaint(base, dir, now), ...listed];
  }
  if (state !== 'triage') return listed;
  const gate = listed.find(i => i.id === 'veil_gate.glb')!;
  return [...linkForRedo(listed), ...SPECS.map(s => triageItem(gate, dir, now, s))];
}

/** A Meshy re-texture of a 35 part split that came back as one mesh, for `?inbox=retexture`. */
function repaintedFern(base: InboxItem, dir: string, now: number): InboxItem {
  const row = (id: string, status: InboxItem['worst'], title: string, detail: string | null) => ({
    id,
    status,
    title,
    detail,
    fixes: [],
  });
  return {
    ...base,
    id: 'fern_clump_retexture.glb',
    name: 'Fern clump',
    file: `${dir}/fern_clump_retexture.glb`,
    modifiedMs: now,
    worst: 'warning',
    link: {
      job: 'fern-clump-3',
      project: 'frostveil',
      asset: 'fern-clump',
      assetName: 'Fern clump',
      provider: 'meshy',
      redoOf: null,
    },
    checks: [
      row('triangles', 'pass', '14.2k triangles', 'Within the 60k per model budget.'),
      row('rig', 'pass', 'No rig, as expected', null),
      row(
        'pbr',
        'warning',
        'PBR maps missing',
        'No normal map. The re-texture asked Meshy for them, so it left them out this time.'
      ),
      row(
        'retexture-parts',
        'warning',
        'Parts gone: v2 has 35, this is one mesh',
        "Meshy's re-texture merged them, so Motion states and explode tracks cannot move them. 15.9k tris on v2, 14.2k here. To keep the parts, discard this and use Bake back on v2 in Inspector > Parts."
      ),
      row('origin', 'pass', 'Origin at the bottom centre', null),
    ],
  };
}

/** A Meshy split with a re-texture chosen, for `?inbox=retexture&parts=retexture`. */
function splitWaitingToPaint(base: InboxItem, dir: string, now: number): InboxItem {
  return {
    ...base,
    id: 'frost_lantern_parts.glb',
    name: 'Frost lantern parts',
    file: `${dir}/frost_lantern_parts.glb`,
    modifiedMs: now - 1000,
    link: {
      job: 'frost-lantern-split-1',
      project: 'frostveil',
      asset: 'frost-lantern',
      assetName: 'Frost lantern parts',
      provider: 'meshy',
      redoOf: null,
    },
  };
}

/** The models discarded before the app opened, for `?inbox=triage` and `?inbox=clear`. */
export function seedDiscarded(state: string | null, dir: string, now: number): DiscardedSeed[] {
  if (state !== 'triage' && state !== 'clear') return [];
  const gate = inboxItems(dir, now).find(i => i.id === 'veil_gate.glb')!;
  return discardedSeeds(dir, now, gate);
}
