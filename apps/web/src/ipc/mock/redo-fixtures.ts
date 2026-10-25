/**
 * `?inbox=redo`: two inbox models that came from sends, ready for Redo. The
 * large sakura came from a Meshy send with a texture prompt, the character
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
      id: 'sakura-2',
      asset: 'sakura',
      assetName: 'Sakura',
      provider: 'meshy',
      images: ['refs/sakura_front.png'],
      meshy: { ...structuredClone(presets.meshy.prop), textureResolution: '4k' },
      prompt: 'Pale pink blossoms, dark wet bark, soft painted look',
      state: 'inbox',
      progress: 1,
      credits: 30,
      remoteId: '0192f1c4-meshy',
      createdAtMs: now - 14 * MIN,
      startedAtMs: now - 14 * MIN + 2_000,
      finishedAtMs: now - 9 * MIN,
      inboxFile: 'sakura_large.glb',
      cancel: 'none',
    }),
    base({
      id: 'character-3',
      asset: 'character',
      assetName: 'Character',
      provider: 'tripo',
      variant: 'character',
      images: ['refs/character-model.png'],
      tripo: { ...structuredClone(presets.tripo.character), textureQuality: 'detailed' },
      state: 'inbox',
      progress: 1,
      credits: 80,
      remoteId: 'b7e2d9a0-tripo',
      createdAtMs: now - 8 * MIN,
      startedAtMs: now - 8 * MIN + 1_000,
      finishedAtMs: now - 3 * MIN,
      inboxFile: 'character.glb',
      message: 'Rigged with Mixamo bones, 2 animations.',
      cancel: 'none',
    }),
  ];
}

/** The inbox with the sakura and the character linked to their sends. */
export function linkForRedo(items: InboxItem[]): InboxItem[] {
  const links: Record<string, InboxItem['link']> = {
    'sakura_large.glb': {
      job: 'sakura-2',
      project: 'moon-island',
      asset: 'sakura',
      assetName: 'Sakura',
      provider: 'meshy',
    },
    'character.glb': {
      job: 'character-3',
      project: 'moon-island',
      asset: 'character',
      assetName: 'Character',
      provider: 'tripo',
    },
  };
  return items.map(i => (links[i.id] ? { ...i, link: links[i.id] ?? null } : i));
}
