import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useBuiltStore } from './use-built-store';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'demo');
});

afterEach(() => {
  clearMocks();
  useBuiltStore.setState({ project: null, items: [], adding: false });
});

describe('useBuiltStore', () => {
  it('lists how each built asset stands', async () => {
    await useBuiltStore.getState().load('frostveil');
    const items = useBuiltStore.getState().items;
    expect(items.map(b => [b.asset, b.state])).toEqual([
      ['laptop', 'current'],
      ['newtons-cradle', 'stale'],
    ]);
  });

  it('rebuilds a stale asset into its next version', async () => {
    await useBuiltStore.getState().load('frostveil');
    await useBuiltStore.getState().rebuild('frostveil', 'newtons-cradle', false);
    expect(useBuiltStore.getState().items.find(b => b.asset === 'newtons-cradle')?.state).toBe(
      'current'
    );
    const cradle = useAppStore
      .getState()
      .workspace?.project?.assets?.find(a => a.id === 'newtons-cradle');
    expect(cradle?.versions.length).toBe(2);
    expect(useAppStore.getState().notice?.title).toBe('Built Newtons cradle v2');
  });

  it('says a current asset is up to date without a build', async () => {
    await useBuiltStore.getState().load('frostveil');
    await useBuiltStore.getState().rebuild('frostveil', 'laptop', false);
    expect(useAppStore.getState().notice?.title).toBe('Laptop is up to date');
  });

  it('adds one and builds it', async () => {
    await useBuiltStore.getState().load('frostveil');
    useBuiltStore.setState({ adding: true });
    await useBuiltStore.getState().add('frostveil', {
      name: 'Desk',
      kind: 'prop',
      script: '/Users/maker/room/desk.py',
      helpers: ['/Users/maker/room/common.py'],
      root: null,
      replace: false,
    });
    expect(useBuiltStore.getState().adding).toBe(false);
    await expect
      .poll(() => useBuiltStore.getState().items.find(b => b.asset === 'desk')?.state)
      .toBe('current');
    const desk = useBuiltStore.getState().items.find(b => b.asset === 'desk');
    expect(desk?.source.helpers).toEqual(['built/common.py']);
  });
});
