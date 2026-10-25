/**
 * Fake Send to 3D for mock mode: no network, no keychain. The configured
 * scenarios open with a Tripo job generating, a By hand send waiting for
 * its model and a Meshy job that ran out of credits; `?agents=active`
 * adds a send an agent asked for, waiting for approval (`?agents=batch`:
 * three of them under one approval). `?handoff=batch` adds the workbench
 * objects, handed off by hand in one batch and back in mixed states. New
 * sends move along on timers and land in the Inbox.
 */
import type {
  Asset,
  AssetStatus,
  BatchItemOptions,
  BatchOptions,
  BatchRequest,
  BatchSent,
  CardRow,
  DownloadMatch,
  DownloadsScan,
  HandoffCard,
  KeyStatus,
  KeyTest,
  MeshyPreset,
  ProjectManifest,
  ProviderEvent,
  ProviderId,
  ProviderJob,
  RefImage,
  SendOptions,
  SendRequest,
  Stage,
  TripoPreset,
  WorkspaceConfig,
} from '../client';
import { HOME } from './fixtures';
import { WORKBENCH_OBJECTS, labRefs, objectRef } from './lab-fixtures';
import { clock } from '@/lib/clock';

type Args = Record<string, unknown>;

/** The fake key tails: Tripo has one saved, Meshy does not. */
const TRIPO_TAIL = '9f2c';

const onOff = (on: boolean) => (on ? 'On' : 'Off');

function tripoRows(p: TripoPreset): CardRow[] {
  return [
    {
      label: 'Model',
      value: p.model.startsWith('v3.1') ? 'H3.1' : p.model,
      note: `model id ${p.model}`,
    },
    {
      label: 'Face limit',
      value: p.faceLimit ? p.faceLimit.toLocaleString('en-US') : 'Adaptive',
      note: null,
    },
    { label: 'Texture', value: onOff(p.texture), note: null },
    { label: 'PBR', value: onOff(p.texture && p.pbr), note: null },
    {
      label: 'Texture quality',
      value: p.textureQuality === 'detailed' ? 'Detailed' : 'Standard',
      note: null,
    },
    {
      label: 'Geometry',
      value: p.geometryQuality === 'detailed' ? 'Ultra' : 'Standard',
      note: null,
    },
    { label: 'Smart low poly', value: onOff(p.smartLowPoly), note: null },
    { label: 'Real-world size', value: onOff(p.autoSize), note: null },
    { label: 'Fix the image first', value: onOff(p.imageAutofix), note: null },
  ];
}

function meshyRows(p: MeshyPreset): CardRow[] {
  return [
    { label: 'AI model', value: p.aiModel === 'latest' ? 'Meshy 7.1' : p.aiModel, note: null },
    { label: 'Texture', value: onOff(p.shouldTexture), note: null },
    { label: 'PBR', value: onOff(p.enablePbr), note: null },
    { label: 'Remesh', value: onOff(p.shouldRemesh), note: null },
    {
      label: 'Target polycount',
      value: p.targetPolycount.toLocaleString('en-US'),
      note: p.shouldRemesh ? null : 'only with Remesh on',
    },
  ];
}

const VIEWS = ['front', 'left', 'back', 'right'];

function card(job: ProviderJob): HandoffCard {
  const tripo = job.service === 'tripo';
  const multi = job.images.length > 1;
  return {
    service: job.service,
    site: tripo ? 'https://studio.tripo3d.ai' : 'https://www.meshy.ai/workspace',
    mode: tripo
      ? multi
        ? 'Multiview to 3D'
        : 'Image to 3D'
      : multi
        ? 'Multi-image to 3D'
        : 'Image to 3D',
    rows: tripo && job.tripo ? tripoRows(job.tripo) : job.meshy ? meshyRows(job.meshy) : [],
    images: job.images.map((_, i) => `${VIEWS[i] ?? `view_${i + 1}`}.png`),
    returnName: `${job.id}.glb`,
  };
}

/** Refs the provider fixtures send, served from the Board renders. */
export const PROVIDER_REF_FILES: Record<string, string> = {
  'refs/sakura_front.png': 'sakura-large.webp',
  'refs/lanterns_front.png': 'stone-lantern.webp',
  'refs/far_spire_front.png': 'far-spire.webp',
};

function refImages(lab: RefImage[]): RefImage[] {
  const extra = Object.keys(PROVIDER_REF_FILES).map(file => ({
    file,
    name: file.slice('refs/'.length),
    bytes: 1.6e6,
  }));
  return [...extra, ...lab.filter(r => !(r.file in PROVIDER_REF_FILES))];
}

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

export function createFakeProviders({
  config,
  configured,
  agentsActive,
  agentsBatch = false,
  handoff = null,
  projectDir,
  project,
  saveProject,
  lab,
  emit,
  fail,
}: {
  config: () => WorkspaceConfig;
  configured: boolean;
  agentsActive: boolean;
  /** Three agent sends wait under one approval instead of one. */
  agentsBatch?: boolean;
  /** `batch`: the workbench objects batch. */
  handoff?: string | null;
  projectDir: (id: string) => string;
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  /** The Lab's refs and what each direction sent, so a send picks them up. */
  lab?: {
    refs: (project: string) => RefImage[];
    sentFor: (project: string, assetName: string) => string[];
  };
  emit?: (event: ProviderEvent) => void;
  fail: (code: string, message: string) => never;
}) {
  const now = clock.nowMs();
  const keys = new Map<ProviderId, string>([['tripo', TRIPO_TAIL]]);
  const timers = new Map<string, ReturnType<typeof setInterval>>();
  let jobs: ProviderJob[] = [];

  const base = (
    partial: Partial<ProviderJob> & Pick<ProviderJob, 'id' | 'asset' | 'assetName' | 'provider'>
  ): ProviderJob => {
    const presets = config().providers;
    const service: ProviderId = partial.provider === 'meshy' ? 'meshy' : 'tripo';
    return {
      project: 'moon-island',
      service,
      variant: 'prop',
      images: [],
      tripo: service === 'tripo' ? structuredClone(presets.tripo.prop) : null,
      meshy: service === 'meshy' ? structuredClone(presets.meshy.prop) : null,
      state: 'queued',
      phase: null,
      progress: 0,
      remoteId: null,
      credits: null,
      createdAtMs: now,
      startedAtMs: null,
      finishedAtMs: null,
      message: null,
      inboxFile: null,
      handoff: null,
      card: null,
      version: null,
      viaAgent: false,
      cancel: 'free',
      batch: null,
      ...partial,
    };
  };

  const handoffDir = (job: Pick<ProviderJob, 'project' | 'id'>) =>
    `${projectDir(job.project)}/handoff/${job.id}`;

  if (configured) {
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
    lanterns.card = card(lanterns);
    jobs = [
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
        job.card = card(job);
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
  }

  const find = (id: string): ProviderJob =>
    jobs.find(j => j.id === id) ?? fail('not-found', `Could not find the job ${id}.`);

  const put = (job: ProviderJob) => {
    jobs = jobs.some(j => j.id === job.id)
      ? jobs.map(j => (j.id === job.id ? job : j))
      : [...jobs, job];
    emit?.({ type: 'job', job: structuredClone(job) });
    return job;
  };

  const touch = (job: ProviderJob, status: AssetStatus) => {
    const p = project(job.project);
    const next = saveProject({
      ...p,
      assets: (p.assets ?? []).map((a: Asset) =>
        a.id === job.asset ? { ...a, stage: 'tripo' as Stage, status } : a
      ),
    });
    emit?.({ type: 'project', project: next });
  };

  /** Moves a fake API job along: upload, generate, download, Inbox. */
  const run = (id: string) => {
    const started = clock.nowMs();
    const first = find(id);
    put({
      ...first,
      state: 'running',
      phase: 'upload',
      progress: 0.04,
      startedAtMs: started,
      // Meshy cancels while its task is pending; Tripo never cancels.
      cancel: first.provider === 'meshy' ? 'provider' : 'none',
    });
    touch(find(id), 'running');
    const timer = setInterval(() => {
      const job = jobs.find(j => j.id === id);
      if (!job || job.state !== 'running') {
        clearInterval(timer);
        timers.delete(id);
        return;
      }
      const progress = Math.min(1, (job.progress ?? 0) + 0.09);
      if (progress >= 1) {
        clearInterval(timer);
        timers.delete(id);
        put({
          ...job,
          state: 'inbox',
          phase: null,
          progress: 1,
          credits: job.provider === 'tripo' ? 30 : 20,
          finishedAtMs: clock.nowMs(),
          inboxFile: `${job.id}.glb`,
        });
        touch(job, 'review');
        return;
      }
      put({ ...job, progress, phase: progress > 0.9 ? 'download' : 'generate' });
    }, 700);
    timers.set(id, timer);
  };

  const nextId = (asset: string) => {
    const used = jobs.filter(j => j.asset === asset).length;
    return `${asset}-${used + 1}`;
  };

  const status = (service: ProviderId): KeyStatus => ({
    service,
    saved: keys.has(service),
    tail: keys.get(service) ?? null,
  });

  /** The asset's own refs, front first: what its direction sent, else refs named after it. */
  const suggestedFor = (p: ProjectManifest, asset: Asset, images: RefImage[]): string[] => {
    const slug = asset.id.replace(/-/g, '_');
    const fromLab = (lab?.sentFor(p.id, asset.name) ?? []).filter(file =>
      images.some(i => i.file === file)
    );
    const byName = images
      .filter(i => i.name.toLowerCase().replace(/-/g, '_').startsWith(slug))
      .map(i => i.file);
    return (fromLab.length > 0 ? fromLab : byName).slice(0, 4);
  };

  const assetOf = (p: ProjectManifest, assetId: string): Asset =>
    (p.assets ?? []).find(a => a.id === assetId) ??
    fail('not-found', `Could not find the asset ${assetId}.`);

  /** A new send of `asset`, By hand ones waiting for the model, API ones on their way. */
  const start = (job: ProviderJob) => {
    if (job.provider === 'by-hand') {
      job.state = 'waiting';
      job.message = 'Waiting for your model';
      job.cancel = 'stop-waiting';
      job.handoff = handoffDir(job);
      job.card = card(job);
      put(job);
      touch(job, 'queued');
      return job;
    }
    put(job);
    touch(job, 'queued');
    setTimeout(() => run(job.id), 600);
    return job;
  };

  const handlers: Record<string, (args: Args) => unknown> = {
    provider_send_options: ({ project: id, asset: assetId }): SendOptions => {
      const p = project(String(id));
      const asset = assetOf(p, String(assetId));
      const images = refImages(lab?.refs(p.id) ?? labRefs());
      return {
        project: p.id,
        projectDir: projectDir(p.id),
        asset,
        images,
        suggested: suggestedFor(p, asset, images),
        variant: asset.kind === 'character' ? 'character' : 'prop',
        keys: [status('tripo'), status('meshy')],
        jobs: jobs.filter(j => j.asset === asset.id),
      };
    },
    provider_batch_options: ({ project: id, assets }): BatchOptions => {
      const p = project(String(id));
      const images = refImages(lab?.refs(p.id) ?? labRefs());
      const items: BatchItemOptions[] = [...new Set(assets as string[])].map(assetId => {
        const asset = assetOf(p, assetId);
        return {
          asset,
          suggested: suggestedFor(p, asset, images),
          variant: asset.kind === 'character' ? 'character' : 'prop',
        };
      });
      return {
        project: p.id,
        projectDir: projectDir(p.id),
        items,
        keys: [status('tripo'), status('meshy')],
      };
    },
    provider_send_batch: ({ request }): BatchSent => {
      const req = request as BatchRequest;
      const p = project(req.project);
      if (req.items.length === 0) fail('invalid', 'Pick at least one asset to send.');
      if (req.items.length > 24) fail('invalid', 'A batch takes at most 24 assets.');
      if (req.provider !== 'by-hand' && !keys.has(req.provider))
        fail('invalid', `Save a ${req.provider === 'tripo' ? 'Tripo' : 'Meshy'} API key first.`);
      const service: ProviderId = req.provider === 'by-hand' ? req.service : req.provider;
      // Checked whole before anything is written.
      const items = req.items.map(item => {
        const asset = assetOf(p, item.asset);
        if (item.images.length === 0) fail('invalid', `${asset.name}: Pick at least one image.`);
        return { item, asset };
      });
      const batch = `batch-${clock.nowMs().toString(16)}`;
      const sent = items.map(({ item, asset }) =>
        start(
          base({
            id: nextId(asset.id),
            project: p.id,
            asset: asset.id,
            assetName: asset.name,
            provider: req.provider,
            service,
            variant: item.variant,
            images: item.images,
            tripo:
              service === 'tripo'
                ? (item.tripo ??
                  (item.variant === 'character' ? req.tripoCharacter : req.tripoProp))
                : null,
            meshy:
              service === 'meshy'
                ? (item.meshy ??
                  (item.variant === 'character' ? req.meshyCharacter : req.meshyProp))
                : null,
            createdAtMs: clock.nowMs(),
            batch,
          })
        )
      );
      return { batch, jobs: sent, failed: [] };
    },
    provider_check_downloads: ({ project: id }): DownloadsScan => {
      const waiting = jobs.filter(
        j => j.project === id && j.provider === 'by-hand' && j.state === 'waiting'
      );
      const since = waiting.length ? Math.min(...waiting.map(j => j.createdAtMs ?? now)) : null;
      const at = clock.nowMs();
      const file = (
        name: string,
        inner: string | null,
        bytes: number,
        ago: number,
        job: ProviderJob | undefined,
        reason: string | null
      ): DownloadMatch => ({
        path: `${HOME}/Downloads/${name}`,
        name,
        inner,
        bytes,
        modifiedMs: at - ago * 60_000,
        job: job?.id ?? null,
        assetName: job?.assetName ?? null,
        reason: job ? reason : null,
      });
      const [first, second] = waiting;
      const slug = (j: ProviderJob) => j.assetName.toLowerCase().replace(/[^a-z0-9]+/g, '_');
      const files = waiting.length
        ? [
            ...(first
              ? [file(`${slug(first)}.glb`, null, 4.2e6, 3, first, `Named like ${first.assetName}`)]
              : []),
            ...(second
              ? [
                  file(
                    `tripo_${slug(second)}_8c1f.zip`,
                    `${slug(second)}.glb`,
                    6.8e6,
                    7,
                    second,
                    `Name close to ${second.assetName}`
                  ),
                ]
              : []),
            file('model (3).glb', null, 3.1e6, 12, undefined, null),
          ]
        : [];
      return { folder: `${HOME}/Downloads`, sinceMs: since, files, waiting };
    },
    provider_open_site: () => null,
    provider_reveal_image: () => null,
    provider_send: ({ request }) => {
      const req = request as SendRequest;
      const p = project(req.project);
      const asset = assetOf(p, req.asset);
      if (req.images.length === 0) fail('invalid', 'Pick at least one image.');
      if (req.provider !== 'by-hand' && !keys.has(req.provider)) {
        fail('invalid', `Save a ${req.provider === 'tripo' ? 'Tripo' : 'Meshy'} API key first.`);
      }
      const job = base({
        id: nextId(asset.id),
        project: p.id,
        asset: asset.id,
        assetName: asset.name,
        provider: req.provider,
        service: req.service,
        variant: req.variant,
        images: req.images,
        tripo: req.service === 'tripo' ? req.tripo : null,
        meshy: req.service === 'meshy' ? req.meshy : null,
        createdAtMs: clock.nowMs(),
      });
      return start(job);
    },
    provider_jobs: ({ project: id }) => jobs.filter(j => !id || j.project === id),
    provider_cancel: ({ id }) => {
      const job = find(String(id));
      if (job.cancel === 'none') {
        fail(
          'invalid',
          job.provider === 'tripo' && job.state === 'running'
            ? 'Tripo cannot cancel a task once it is sent, so this one will finish and arrive in the Inbox. The credits are spent.'
            : 'This job is already over.'
        );
      }
      const message =
        job.cancel === 'stop-waiting'
          ? 'Stopped waiting for this model.'
          : job.cancel === 'provider'
            ? 'Cancelled before Meshy started it. Meshy refunds the credits.'
            : 'Cancelled. Nothing was spent.';
      const next = put({
        ...job,
        state: 'cancelled',
        finishedAtMs: clock.nowMs(),
        message,
        cancel: 'none',
      });
      touch(job, 'queued');
      return next;
    },
    provider_hand_back: ({ id, file }) => {
      const job = find(String(id));
      if (job.state !== 'waiting') fail('invalid', 'This send is not waiting for a model.');
      if (!/\.(glb|zip)$/i.test(String(file)))
        fail('invalid', 'That is not a GLB file, or a zip with one inside.');
      const next = put({
        ...job,
        state: 'inbox',
        finishedAtMs: clock.nowMs(),
        inboxFile: `${job.id}.glb`,
        message: null,
        cancel: 'none',
      });
      touch(job, 'review');
      return next;
    },
    provider_clear_finished: ({ project: id }) => {
      jobs = jobs.filter(
        j =>
          j.project !== id ||
          ['approval', 'queued', 'running', 'waiting', 'inbox'].includes(j.state)
      );
      return jobs.filter(j => j.project === id);
    },
    provider_open_handoff: () => null,
    provider_keys: () => [status('tripo'), status('meshy')],
    provider_key_save: ({ service, key }) => {
      const text = String(key).trim();
      if (!text || /\s/.test(text)) fail('invalid', 'That does not look like an API key.');
      keys.set(service as ProviderId, text.slice(-4));
      return status(service as ProviderId);
    },
    provider_key_remove: ({ service }) => {
      keys.delete(service as ProviderId);
      return status(service as ProviderId);
    },
    provider_key_test: ({ service, key }): KeyTest => {
      const id = service as ProviderId;
      const name = id === 'tripo' ? 'Tripo' : 'Meshy';
      if (!key && !keys.has(id)) fail('invalid', `Save a ${name} API key first.`);
      return {
        service: id,
        ok: true,
        balance: id === 'tripo' ? 1240 : 380,
        message: `The key works. ${id === 'tripo' ? '1,240' : '380'} credits left.`,
      };
    },
  };

  /** An agent's send was approved or denied in the app. */
  const decide = (jobId: string, approve: boolean) => {
    const job = jobs.find(j => j.id === jobId);
    if (!job || job.state !== 'approval') return;
    if (approve) {
      put({ ...job, state: 'queued', message: null });
      setTimeout(() => run(job.id), 400);
    } else {
      put({
        ...job,
        state: 'cancelled',
        finishedAtMs: clock.nowMs(),
        message: 'Denied in Maquette. Nothing was spent.',
        cancel: 'none',
      });
      touch(job, 'queued');
    }
  };

  return { handlers, decide, jobs: () => jobs };
}
