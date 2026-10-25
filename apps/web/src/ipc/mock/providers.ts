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
  ProjectManifest,
  ProviderEvent,
  ProviderId,
  ProviderJob,
  RefImage,
  SendOptions,
  SendRequest,
  Stage,
  WorkspaceConfig,
} from '../client';
import { clock } from '@/lib/clock';
import type { Args } from './backend-options';
import { createKeyStore } from './create-key-store';
import { handoffCard } from './handoff-card';
import { jobFactory } from './job-factory';
import { labRefs } from './lab-fixtures';
import { PROVIDER_REF_FILES, refImages, suggestedRefs } from './provider-refs';
import { seedProviders } from './seed-providers';
import { scanDownloads } from './scan-downloads';

export { PROVIDER_REF_FILES };

export function createFakeProviders({
  config,
  configured,
  agentsActive,
  agentsBatch = false,
  handoff = null,
  inbox = null,
  projectDir,
  project,
  saveProject,
  lab,
  emit,
  fail,
  keys,
}: {
  config: () => WorkspaceConfig;
  configured: boolean;
  agentsActive: boolean;
  /** Three agent sends wait under one approval instead of one. */
  agentsBatch?: boolean;
  /** `batch`: the workbench objects batch. */
  handoff?: string | null;
  /** `redo`: the sends behind two inbox models. */
  inbox?: string | null;
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
  /** The services with a saved key; Tripo alone by default. */
  keys?: readonly ProviderId[];
}) {
  const now = clock.nowMs();
  const timers = new Map<string, ReturnType<typeof setInterval>>();
  const keyStore = createKeyStore(keys);
  let jobs: ProviderJob[] = [];

  const base = jobFactory(config, now);

  const handoffDir = (job: Pick<ProviderJob, 'project' | 'id'>) =>
    `${projectDir(job.project)}/handoff/${job.id}`;

  if (configured) {
    jobs = seedProviders({
      base,
      config,
      now,
      agentsActive,
      agentsBatch,
      handoff,
      inbox,
      handoffDir,
      project,
      saveProject,
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
      job.card = handoffCard(job);
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
        suggested: suggestedRefs(lab, p, asset, images),
        variant: asset.kind === 'character' ? 'character' : 'prop',
        keys: [keyStore.status('tripo'), keyStore.status('meshy')],
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
          suggested: suggestedRefs(lab, p, asset, images),
          variant: asset.kind === 'character' ? 'character' : 'prop',
        };
      });
      return {
        project: p.id,
        projectDir: projectDir(p.id),
        items,
        keys: [keyStore.status('tripo'), keyStore.status('meshy')],
      };
    },
    provider_send_batch: ({ request }): BatchSent => {
      const req = request as BatchRequest;
      const p = project(req.project);
      if (req.items.length === 0) fail('invalid', 'Pick at least one asset to send.');
      if (req.items.length > 24) fail('invalid', 'A batch takes at most 24 assets.');
      if (req.provider !== 'by-hand' && !keyStore.has(req.provider))
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
            prompt: item.prompt ?? null,
            redoOf: item.redoOf ?? null,
          })
        )
      );
      return { batch, jobs: sent, failed: [] };
    },
    provider_check_downloads: ({ project: id }) => scanDownloads(jobs, id, now),
    provider_open_site: () => null,
    provider_reveal_image: () => null,
    provider_send: ({ request }) => {
      const req = request as SendRequest;
      const p = project(req.project);
      const asset = assetOf(p, req.asset);
      if (req.images.length === 0) fail('invalid', 'Pick at least one image.');
      if (req.provider !== 'by-hand' && !keyStore.has(req.provider)) {
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
        prompt: req.prompt ?? null,
        redoOf: req.redoOf ?? null,
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
    ...keyStore.handlers,
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

  return { handlers, decide, jobs: () => jobs, hasKey: keyStore.has };
}
