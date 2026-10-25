import type {
  Asset,
  AssetStatus,
  ProjectManifest,
  ProviderJob,
  Stage,
  WorkspaceConfig,
} from '../client';
import { handoffCard } from './handoff-card';
import type { JobPartial } from './job-factory';
import { redoJobs } from './redo-fixtures';
import { WORKBENCH_OBJECTS, objectRef } from './lab-fixtures';

/** The batch the `?handoff=batch` fixture wrote, and how far each object got. */
const WORKBENCH_BATCH = 'batch-19a2f0c1e4b';
const WORKBENCH_STATES: Record<string, { state: ProviderJob['state']; variant?: 'character' }> = {
  'Desk lamp': { state: 'waiting' },
  'Sculptor figure': { state: 'waiting', variant: 'character' },
  'Clay island': { state: 'inbox' },
  'Turntable island': { state: 'accepted' },
  'Brush cup': { state: 'waiting' },
  Calipers: { state: 'waiting' },
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
  const lanterns = base({
    id: 'lanterns-1',
    asset: 'lanterns',
    assetName: 'Stone lanterns',
    provider: 'by-hand',
    images: ['refs/lanterns_front.png'],
    state: 'waiting',
    createdAtMs: now - 26 * 60_000,
    message: 'Waiting for your model',
    cancel: 'stop-waiting',
  });
  lanterns.handoff = handoffDir(lanterns);
  lanterns.card = handoffCard(lanterns);
  const jobs: ProviderJob[] = [
    base({
      id: 'sakura-1',
      asset: 'sakura',
      assetName: 'Sakura',
      provider: 'tripo',
      images: ['refs/sakura_front.png'],
      state: 'running',
      phase: 'generate',
      progress: 0.46,
      remoteId: 'a1f0c3d2-tripo',
      createdAtMs: now - 80_000,
      startedAtMs: now - 72_000,
      cancel: 'none',
    }),
    lanterns,
    base({
      id: 'far-spire-1',
      asset: 'far-spire',
      assetName: 'Far spire',
      provider: 'meshy',
      images: ['refs/far_spire_front.png'],
      state: 'failed',
      phase: 'generate',
      createdAtMs: now - 3 * 3_600_000,
      startedAtMs: now - 3 * 3_600_000 + 2_000,
      finishedAtMs: now - 3 * 3_600_000 + 9_000,
      message: 'Meshy says the account is out of credits. Top up, then send again.',
      cancel: 'none',
    }),
    base({
      id: 'character-2',
      asset: 'character',
      assetName: 'Character',
      provider: 'tripo',
      variant: 'character',
      images: ['refs/character-model.png'],
      tripo: structuredClone(config().providers.tripo.character),
      state: 'accepted',
      progress: 1,
      credits: 70,
      createdAtMs: now - 2 * 86_400_000,
      startedAtMs: now - 2 * 86_400_000 + 1_000,
      finishedAtMs: now - 2 * 86_400_000 + 4 * 60_000 + 12_000,
      inboxFile: 'character-2.glb',
      version: 2,
      message: 'Rigged with Mixamo bones, 2 animations.',
      cancel: 'none',
    }),
  ];
  if (agentsActive) {
    const asked = [
      { id: 'far-spire-2', asset: 'far-spire', assetName: 'Far spire', image: 'far_spire_front' },
      ...(agentsBatch
        ? [
            { id: 'sakura-2', asset: 'sakura', assetName: 'Sakura', image: 'sakura_front' },
            {
              id: 'lanterns-2',
              asset: 'lanterns',
              assetName: 'Stone lanterns',
              image: 'lanterns_front',
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
    WORKBENCH_OBJECTS.forEach((o, i) => {
      const spec = WORKBENCH_STATES[o.title] ?? { state: 'waiting' as const };
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
        batch: WORKBENCH_BATCH,
      });
      job.handoff = handoffDir(job);
      job.card = handoffCard(job);
      jobs.push(job);
    });
  }
  // The Board shows the sends: assets move to the Tripo column with their state.
  const status: Record<string, [Stage, AssetStatus]> = {
    sakura: ['tripo', 'running'],
    lanterns: ['tripo', 'queued'],
    'far-spire': ['tripo', agentsActive ? 'queued' : 'failed'],
  };
  const p = project('moon-island');
  const at = new Date(now - 52 * 60_000).toISOString();
  const objects: Asset[] =
    handoff === 'batch'
      ? WORKBENCH_OBJECTS.map(o => {
          const spec = WORKBENCH_STATES[o.title];
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
