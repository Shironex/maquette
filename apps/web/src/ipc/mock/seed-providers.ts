import type {
  Asset,
  AssetStatus,
  ProjectManifest,
  ProviderJob,
  Stage,
  StudioSettings,
  WorkspaceConfig,
} from '../client';
import { handoffCard } from './handoff-card';
import type { JobPartial } from './job-factory';
import { redoJobs } from './redo-fixtures';
import { VEIL_GATE_OBJECTS, objectRef } from './lab-fixtures';

/** The batch the `?handoff=batch` fixture wrote, and how far each object got. */
const VEIL_GATE_BATCH = 'batch-19a2f0c1e4b';
const VEIL_GATE_STATES: Record<
  string,
  { state: ProviderJob['state']; variant?: 'character'; studio?: StudioSettings }
> = {
  'Lantern post': { state: 'waiting', studio: { mesh: 'smart-mesh', rig: 'none' } },
  'Hooded guardian': {
    state: 'waiting',
    variant: 'character',
    studio: { mesh: 'hd-model', rig: 'biped' },
  },
  'Ice shards': { state: 'inbox' },
  'Orb shrine': { state: 'accepted' },
  'Sigil door': { state: 'waiting' },
  'Step ring': { state: 'waiting' },
};

const slugOf = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');

interface SeedContext {
  base: (partial: JobPartial) => ProviderJob;
  config: () => WorkspaceConfig;
  now: number;
  agentsActive: boolean;
  agentsBatch: boolean;
  handoff: string | null;
  /** `redo`: the sends behind two inbox models, for Redo. */
  inbox?: string | null;
  handoffDir: (job: Pick<ProviderJob, 'project' | 'id'>) => string;
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
}

/** The jobs a configured workspace opens with; the Board's assets follow their state. */
export function seedProviders({
  base,
  config,
  now,
  agentsActive,
  agentsBatch,
  handoff,
  inbox = null,
  handoffDir,
  project,
  saveProject,
}: SeedContext): ProviderJob[] {
  const guardians = base({
    id: 'guardian-statues-1',
    asset: 'guardian-statues',
    assetName: 'Guardian statues',
    provider: 'by-hand',
    images: ['refs/guardian_statues_front.png'],
    state: 'waiting',
    createdAtMs: now - 26 * 60_000,
    message: 'Waiting for your model',
    cancel: 'stop-waiting',
    studio: { mesh: 'hd-model', rig: 'none' },
  });
  guardians.handoff = handoffDir(guardians);
  guardians.card = handoffCard(guardians);
  const jobs: ProviderJob[] = [
    base({
      id: 'veil-gate-1',
      asset: 'veil-gate',
      assetName: 'Veil gate',
      provider: 'tripo',
      images: ['refs/veil_gate_front.png'],
      state: 'running',
      phase: 'generate',
      progress: 0.46,
      remoteId: 'a1f0c3d2-tripo',
      createdAtMs: now - 80_000,
      startedAtMs: now - 72_000,
      cancel: 'none',
    }),
    guardians,
    base({
      id: 'rune-obelisk-1',
      asset: 'rune-obelisk',
      assetName: 'Rune obelisk',
      provider: 'meshy',
      images: ['refs/rune_obelisk_front.png'],
      state: 'failed',
      phase: 'generate',
      createdAtMs: now - 3 * 3_600_000,
      startedAtMs: now - 3 * 3_600_000 + 2_000,
      finishedAtMs: now - 3 * 3_600_000 + 9_000,
      message: 'Meshy says the account is out of credits. Top up, then send again.',
      cancel: 'none',
    }),
    base({
      id: 'elsheria-2',
      asset: 'elsheria',
      assetName: 'Elsheria',
      provider: 'tripo',
      variant: 'character',
      images: ['refs/elsheria-model.png'],
      tripo: structuredClone(config().providers.tripo.character),
      state: 'accepted',
      progress: 1,
      credits: 70,
      createdAtMs: now - 2 * 86_400_000,
      startedAtMs: now - 2 * 86_400_000 + 1_000,
      finishedAtMs: now - 2 * 86_400_000 + 4 * 60_000 + 12_000,
      inboxFile: 'elsheria-2.glb',
      version: 2,
      message: 'Rigged with Mixamo bones, 2 animations.',
      cancel: 'none',
    }),
  ];
  if (agentsActive) {
    const asked = [
      {
        id: 'rune-obelisk-2',
        asset: 'rune-obelisk',
        assetName: 'Rune obelisk',
        image: 'rune_obelisk_front',
      },
      ...(agentsBatch
        ? [
            {
              id: 'veil-gate-2',
              asset: 'veil-gate',
              assetName: 'Veil gate',
              image: 'veil_gate_front',
            },
            {
              id: 'guardian-statues-2',
              asset: 'guardian-statues',
              assetName: 'Guardian statues',
              image: 'guardian_statues_front',
            },
          ]
        : []),
    ];
    for (const a of asked)
      jobs.push(
        base({
          id: a.id,
          asset: a.asset,
          assetName: a.assetName,
          provider: 'tripo',
          images: [`refs/${a.image}.png`],
          state: 'approval',
          createdAtMs: now - 40_000,
          message: 'Waiting for your approval before spending credits',
          viaAgent: true,
          batch: agentsBatch ? 'batch-agent-1' : null,
        })
      );
  }
  if (inbox === 'redo') jobs.push(...redoJobs(base, config, now));
  if (handoff === 'batch') {
    const written = now - 52 * 60_000;
    VEIL_GATE_OBJECTS.forEach((o, i) => {
      const spec = VEIL_GATE_STATES[o.title] ?? { state: 'waiting' as const };
      const variant = spec.variant ?? 'prop';
      const back = spec.state !== 'waiting';
      const job = base({
        id: `${slugOf(o.title)}-1`,
        asset: slugOf(o.title),
        assetName: o.title,
        provider: 'by-hand',
        variant,
        images: [objectRef(o.title)],
        tripo: structuredClone(config().providers.tripo[variant]),
        state: spec.state,
        createdAtMs: written + i * 1_000,
        finishedAtMs: back ? now - (30 - i * 4) * 60_000 : null,
        message: back ? null : 'Waiting for your model',
        inboxFile: back ? `${slugOf(o.title)}-1.glb` : null,
        version: spec.state === 'accepted' ? 1 : null,
        cancel: back ? 'none' : 'stop-waiting',
        batch: VEIL_GATE_BATCH,
        studio: spec.studio ?? {
          mesh: 'hd-model',
          rig: variant === 'character' ? 'biped' : 'none',
        },
      });
      job.handoff = handoffDir(job);
      job.card = handoffCard(job);
      jobs.push(job);
    });
  }
  // The Board shows the sends: assets move to the Tripo column with their state.
  const status: Record<string, [Stage, AssetStatus]> = {
    'veil-gate': ['tripo', 'running'],
    'guardian-statues': ['tripo', 'queued'],
    'rune-obelisk': ['tripo', agentsActive ? 'queued' : 'failed'],
  };
  const p = project('frostveil');
  const at = new Date(now - 52 * 60_000).toISOString();
  const objects: Asset[] =
    handoff === 'batch'
      ? VEIL_GATE_OBJECTS.map(o => {
          const spec = VEIL_GATE_STATES[o.title];
          const accepted = spec?.state === 'accepted';
          return {
            id: slugOf(o.title),
            name: o.title,
            kind: spec?.variant ?? 'prop',
            stage: accepted ? 'blender' : 'tripo',
            status:
              spec?.state === 'inbox' ? 'review' : accepted ? 'done' : ('queued' as AssetStatus),
            versions: accepted
              ? [
                  {
                    version: 1,
                    file: `models/${slugOf(o.title).replace(/-/g, '_')}_v1.glb`,
                    createdAt: at,
                    stats: null,
                  },
                ]
              : [],
            thumbnail: objectRef(o.title),
            createdAt: at,
            updatedAt: at,
          };
        })
      : [];
  saveProject({
    ...p,
    assets: [
      ...(p.assets ?? []).map(a => {
        const next = status[a.id];
        return next ? { ...a, stage: next[0], status: next[1] } : a;
      }),
      ...objects,
    ],
  });
  return jobs;
}
