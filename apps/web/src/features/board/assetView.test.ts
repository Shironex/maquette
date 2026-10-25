import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { assetView } from './assetView';
import { mockAsset } from './fakeBoardEnv';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, 'batch');
  await useProvidersStore.getState().load('moon-island');
});

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], busy: [] });
});

describe('assetView', () => {
  it('reads the picture, the latest send and a waiting By hand job', () => {
    const env = {
      workspace: useAppStore.getState().workspace,
      jobs: useProvidersStore.getState().jobs,
    };
    const cup = assetView(mockAsset('Brush cup'), env);
    expect(cup.waiting?.id).toBe('brush-cup-1');
    expect(cup.sent?.label).toBe('Waiting for your model');
    const lamp = assetView(mockAsset('Desk lamp'), env);
    expect(lamp.refPicture).toBe('refs/desk_lamp.png');
    expect(assetView(mockAsset('Character'), { workspace: null, jobs: [] })).toMatchObject({
      thumb: undefined,
      sent: null,
      waiting: undefined,
    });
  });
});
