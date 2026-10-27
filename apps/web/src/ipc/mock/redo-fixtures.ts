/**
 * `?inbox=redo`: two inbox models that came from sends, ready for Redo. The
 * veil gate came from a Meshy send with a texture prompt, Elsheria
 * from a rigged Tripo send with detailed textures; both keys are saved.
 */
import type { InboxItem, ProviderJob, WorkspaceConfig } from '../client';
import type { JobPartial } from './job-factory';

const MIN = 60_000;

/** The sends behind the redo fixture's inbox models. */
export function redoJobs(
  base: (partial: JobPartial) => ProviderJob,
  config: () => WorkspaceConfig,
  now: number
): ProviderJob[] {
  const presets = config().providers;
  return [
    base({
      id: 'veil-gate-2',
      asset: 'veil-gate',
      assetName: 'Veil gate',
      provider: 'meshy',
      images: ['refs/veil_gate_front.png'],
      meshy: { ...structuredClone(presets.meshy.prop), textureResolution: '4k' },
      prompt: 'Pale frosted stone, violet rune glow, soft painted look',
      state: 'inbox',
      progress: 1,
      credits: 30,
      remoteId: '0192f1c4-meshy',
      createdAtMs: now - 14 * MIN,
      startedAtMs: now - 14 * MIN + 2_000,
      finishedAtMs: now - 9 * MIN,
      inboxFile: 'veil_gate.glb',
      cancel: 'none',
    }),
    base({
      id: 'elsheria-3',
      asset: 'elsheria',
      assetName: 'Elsheria',
      provider: 'tripo',
      variant: 'character',
      images: ['refs/elsheria-model.png'],
      tripo: { ...structuredClone(presets.tripo.character), textureQuality: 'detailed' },
      state: 'inbox',
      progress: 1,
      credits: 80,
      remoteId: 'b7e2d9a0-tripo',
      createdAtMs: now - 8 * MIN,
      startedAtMs: now - 8 * MIN + 1_000,
      finishedAtMs: now - 3 * MIN,
      inboxFile: 'elsheria.glb',
      message: 'Rigged with Mixamo bones, 2 animations.',
      cancel: 'none',
    }),
  ];
}

/** The inbox with the veil gate and Elsheria linked to their sends. */
export function linkForRedo(items: InboxItem[]): InboxItem[] {
  const links: Record<string, InboxItem['link']> = {
    'veil_gate.glb': {
      job: 'veil-gate-2',
      project: 'frostveil',
      asset: 'veil-gate',
      assetName: 'Veil gate',
      provider: 'meshy',
    },
    'elsheria.glb': {
      job: 'elsheria-3',
      project: 'frostveil',
      asset: 'elsheria',
      assetName: 'Elsheria',
      provider: 'tripo',
    },
  };
  return items.map(i => (links[i.id] ? { ...i, link: links[i.id] ?? null } : i));
}
