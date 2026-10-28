import { afterEach, describe, expect, it } from 'vitest';
import { waitFor } from '@testing-library/react';
import type { Arrival } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useDownloadsStore } from './use-downloads-store';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({
    status: null,
    pickerOpen: false,
    chooserFor: null,
    previews: {},
    busy: [],
  });
  useAppStore.setState({ notice: null });
});

const arrival = (over: Partial<Arrival>): Arrival => ({
  id: 'a1',
  path: '/Users/maker/Downloads/maq_guardian-statues_v1_tripo.glb',
  name: 'maq_guardian-statues_v1_tripo.glb',
  inner: null,
  bytes: 1000,
  atMs: 0,
  state: 'imported',
  job: 'guardian-statues-1',
  project: 'frostveil',
  asset: 'guardian-statues',
  assetName: 'Lanterns',
  reason: 'Named for Lanterns',
  inboxFile: 'guardian-statues-1.glb',
  ...over,
});

describe('the Downloads store', () => {
  it('reads the watch and refuses a folder that is not there', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    await useDownloadsStore.getState().load();
    expect(useDownloadsStore.getState().status).toMatchObject({ enabled: true, custom: false });
    await useDownloadsStore.getState().setWatch(true, '/Users/maker/missing');
    expect(useAppStore.getState().notice?.title).toBe('Could not change the Downloads watch');
    expect(useDownloadsStore.getState().saving).toBe(false);
  });

  it('says quietly when a model arrives, and offers the picker when it asks', async () => {
    await openMockWorkspace('configured');
    useDownloadsStore.getState().onArrival(arrival({}));
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'done',
      title: 'Lanterns is in the Inbox',
    });
    expect(useAppStore.getState().notice?.action).toBeUndefined();

    useDownloadsStore.getState().onArrival(arrival({ id: 'a2', state: 'unmatched', job: null }));
    const notice = useAppStore.getState().notice;
    expect(notice?.action?.label).toBe('Choose');
    notice?.action?.run();
    expect(useDownloadsStore.getState().pickerOpen).toBe(true);
  });

  it('offers a look for a close name, and Choose when it cannot tell', async () => {
    await openMockWorkspace('configured');
    useDownloadsStore
      .getState()
      .onArrival(arrival({ state: 'review', reason: 'Name close to Lanterns' }));
    expect(useAppStore.getState().notice).toMatchObject({ title: 'Is this Lanterns?' });
    expect(useAppStore.getState().notice?.action?.label).toBe('Look');
  });

  it('attaches several downloads in a row, moving the chooser on, then closes', async () => {
    await openMockWorkspace('configured', undefined, 'batch', undefined, 'look');
    const store = useDownloadsStore.getState;
    await waitFor(async () => {
      await store().load();
      expect(store().status?.arrivals).toHaveLength(3);
    });
    store().openPicker();
    const [arch, rune] = store().status?.arrivals.filter(a => a.state === 'unmatched') ?? [];
    store().openChooser(arch!.id);
    expect(await store().attach(arch!.id, 'sigil-door')).toBe(true);
    expect(store().chooserFor).toBe(rune!.id);
    // It goes to the Inbox for the asset; nothing is accepted yet.
    expect(store().status?.arrivals.find(a => a.id === arch!.id)).toMatchObject({
      state: 'attached',
      asset: 'sigil-door',
      inboxFile: expect.stringMatching(/\.glb$/),
    });
    expect(
      useAppStore.getState().workspace?.project?.assets?.find(a => a.id === 'sigil-door')?.versions
    ).toHaveLength(0);
    expect(await store().attach(rune!.id, 'step-ring')).toBe(true);
    expect(store().chooserFor).toBeNull();
    expect(store().pickerOpen).toBe(true);
    // The last one, a look: Not this one closes the picker with nothing left.
    const look = store().status?.arrivals.find(a => a.state === 'review');
    await store().dismiss(look!.id);
    expect(store().status?.arrivals.find(a => a.id === look!.id)?.reason).toBe(
      'You said it is not Lantern post'
    );
    expect(store().pickerOpen).toBe(false);
  });

  it('refuses an attach while one is on its way, and says why one fails', async () => {
    await openMockWorkspace('configured');
    expect(await useDownloadsStore.getState().attach('nope', 'sigil-door')).toBe(false);
    expect(useAppStore.getState().notice?.title).toBe('Could not attach that model');
    expect(useDownloadsStore.getState().busy).toEqual([]);
  });
});
