/**
 * OpenAI's Image API in mock mode (`?images=`):
 * - unset (default): no key saved, every kind of job on Codex.
 * - `on`: a key ending in `k7Qm` saved, concepts on OpenAI Images at low
 *   quality, $3.42 of the $10 cap spent this month.
 * - `cap`: like `on`, with $9.96 spent, so a round of four would pass the
 *   cap and Generate says so.
 *
 * Concepts generated while Settings picks OpenAI Images come back marked
 * with that backend, and each one made adds its estimate to the month.
 * Test this key only "lists the models": a key with `bad` in it fails.
 */
import type {
  ConceptEvent,
  ConceptJob,
  ImageApiStatus,
  ImageSettings,
  LabView,
  NewConcepts,
  OpenAiKeyStatus,
  OpenAiKeyTest,
  WorkspaceConfig,
} from '../client';
import { fail, type Args } from './backend-options';

const MODEL = 'gpt-image-2.5-sunburst';
const ESTIMATES = { low: 0.011, medium: 0.058, high: 0.216 };
const SPENT: Record<string, number> = { on: 3.42, cap: 9.96 };

type Handler = (args: Args) => unknown;

interface ImageApiOptions {
  state: string | null;
  config: { get: () => WorkspaceConfig; set: (next: WorkspaceConfig) => void };
  /** The fake Lab's handlers, wrapped to mark and price API jobs. */
  lab: Record<string, Handler>;
}

const settingsOf = (config: WorkspaceConfig): Required<ImageSettings> => {
  const images = config.lab.images;
  return {
    backends: {
      concepts: 'codex',
      objectRefs: 'codex',
      otherViews: 'codex',
      cleanBackground: 'codex',
      ...images?.backends,
    },
    quality: images?.quality ?? 'low',
    monthlyCapUsd: images?.monthlyCapUsd ?? 10,
  };
};

export function createImageApiHandlers({ state, config, lab }: ImageApiOptions) {
  const on = state === 'on' || state === 'cap';
  let tail: string | null = on ? 'k7Qm' : null;
  let spent = SPENT[state ?? ''] ?? 0;
  /** Jobs on the Image API, with their estimate while they are not done. */
  const api = new Map<string, number>();

  if (on) {
    const current = config.get();
    const backends = { ...settingsOf(current).backends, concepts: 'openai-images' as const };
    config.set({
      ...current,
      lab: { ...current.lab, images: { ...settingsOf(current), backends } },
    });
  }

  const perImage = () => ESTIMATES[settingsOf(config.get()).quality];
  const pending = () => [...api.values()].reduce((sum, usd) => sum + usd, 0);
  const key = (): OpenAiKeyStatus => ({ saved: tail !== null, tail });

  /** A Lab job as the app sees it: marked when it runs on the Image API. */
  function stamp(job: ConceptJob): ConceptJob {
    if (!api.has(job.id)) return job;
    if (job.state === 'done' && (api.get(job.id) ?? 0) > 0) {
      spent += api.get(job.id) ?? 0;
      api.set(job.id, 0);
    } else if (job.state !== 'queued' && job.state !== 'running') api.set(job.id, 0);
    return { ...job, backend: 'openai-images' };
  }

  const stampView = (view: LabView): LabView => ({ ...view, jobs: view.jobs.map(stamp) });

  const status = (): ImageApiStatus => ({
    settings: settingsOf(config.get()),
    key: key(),
    monthSpentUsd: spent,
    pendingUsd: pending(),
    model: MODEL,
    estimates: ESTIMATES,
    pricesChecked: '2026-10-28',
    pricesUrl: 'https://developers.openai.com/api/docs/pricing',
  });

  const handlers: Record<string, Handler> = {
    lab_get: args => stampView(lab.lab_get?.(args) as LabView),
    lab_generate: args => {
      const settings = settingsOf(config.get());
      if (settings.backends.concepts !== 'openai-images') return lab.lab_generate?.(args);
      const { count } = args.request as NewConcepts;
      const usd = perImage() * count;
      if (!tail) fail('invalid', 'Save an OpenAI API key in Settings > Image generation first.');
      if (spent + pending() + usd > settings.monthlyCapUsd) {
        fail(
          'invalid',
          `This round would pass the monthly OpenAI image cap of $${settings.monthlyCapUsd}. Raise it in Settings > Image generation.`
        );
      }
      const { project } = args.request as NewConcepts;
      const before = new Set((lab.lab_get?.({ project }) as LabView).jobs.map(j => j.id));
      const view = lab.lab_generate?.(args) as LabView;
      for (const job of view.jobs) if (!before.has(job.id)) api.set(job.id, perImage());
      return stampView(view);
    },
    image_api_status: () => status(),
    image_api_key_save: ({ key: text }) => {
      const value = String(text).trim();
      if (!value || /\s/.test(value)) fail('invalid', 'That does not look like an API key.');
      tail = value.slice(-4);
      return key();
    },
    image_api_key_remove: () => {
      tail = null;
      return key();
    },
    image_api_key_test: ({ key: text }): OpenAiKeyTest => {
      const value = typeof text === 'string' ? text.trim() : '';
      if (!value && !tail) fail('invalid', 'Save an OpenAI API key first.');
      if (value.includes('bad')) {
        return {
          ok: false,
          hasModel: false,
          message: 'OpenAI did not accept the API key. Check it in Settings > Image generation.',
        };
      }
      return { ok: true, hasModel: true, message: `The key works and can use ${MODEL}.` };
    },
  };

  return {
    handlers,
    /** A Lab event as the app sees it. */
    event: (payload: ConceptEvent): ConceptEvent =>
      payload.type === 'job' ? { ...payload, job: stamp(payload.job) } : payload,
  };
}
