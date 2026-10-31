import { afterEach, describe, expect, it } from 'vitest';
import type { InboxItem, ProviderJob } from '@/ipc/client';
import { retextureOffer } from './retexture-offer';
import { useAppStore } from './use-app-store';
import { useInspectorStore } from './use-inspector-store';
import { useProvidersStore } from './use-providers-store';

const item = {
  link: { job: 'split-1', project: 'frostveil', asset: 'fern' },
} as unknown as InboxItem;

function seed(after: string) {
  const job = { id: 'split-1', split: { after } } as unknown as ProviderJob;
  useProvidersStore.setState({ jobs: [job] });
}

afterEach(() => useProvidersStore.setState({ jobs: [] }));

describe('retextureOffer', () => {
  it('says the re-texture waits and links to the Parts tab', () => {
    seed('retexture');
    const offer = retextureOffer(item, 2);
    expect(offer?.detail).toMatch(/not sent yet/);
    offer?.action?.run();
    expect(useAppStore.getState().view).toBe('inspector');
    expect(useInspectorStore.getState().tab).toBe('parts');
    expect(useInspectorStore.getState().selected).toBe('asset/fern/v2');
  });

  it('says nothing for the other choices or a model without a send', () => {
    seed('bake');
    expect(retextureOffer(item, 2)).toBeNull();
    expect(retextureOffer({ link: null } as unknown as InboxItem, 2)).toBeNull();
    expect(retextureOffer(null, 2)).toBeNull();
  });
});
