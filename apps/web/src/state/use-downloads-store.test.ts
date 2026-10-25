import { afterEach, describe, expect, it } from 'vitest';
import type { Arrival } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useDownloadsStore } from './use-downloads-store';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({ status: null, pickerOpen: false, choices: {}, busy: [] });
  useAppStore.setState({ notice: null });
});

const arrival = (over: Partial<Arrival>): Arrival => ({
  id: 'a1',
  path: '/Users/maker/Downloads/maq_lanterns_v1_tripo.glb',
  name: 'maq_lanterns_v1_tripo.glb',
  inner: null,
  bytes: 1000,
  atMs: 0,
  state: 'imported',
  job: 'lanterns-1',
  project: 'moon-island',
  assetName: 'Lanterns',
  reason: 'Named for Lanterns',
  inboxFile: 'lanterns-1.glb',
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

  it('does nothing on Add to inbox before a send is picked', async () => {
    await openMockWorkspace('configured');
    await useDownloadsStore.getState().assign('a1');
    expect(useDownloadsStore.getState().busy).toEqual([]);
    expect(useAppStore.getState().notice).toBeNull();
  });
});
