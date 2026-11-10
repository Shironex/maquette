import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useBuiltStore } from './use-built-store';

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'demo');
});

afterEach(() => {
  clearMocks();
  useBuiltStore.setState({ project: null, items: [], adding: false, replacing: null });
  useAppStore.setState({ notice: null });
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

describe('a built asset in the place of a model', () => {
  const assets = () => useAppStore.getState().workspace?.project?.assets ?? [];

  it('replaces a model, keeps the asset id and can bring the old model back', async () => {
    clearMocks();
    await openMockWorkspace('configured', undefined, undefined, undefined, undefined, 'swap');
    await useBuiltStore.getState().load('frostveil');
    useBuiltStore.setState({
      replacing: { mode: 'replace', asset: 'crystal-lantern', built: 'lantern-rebuilt' },
    });

    await useBuiltStore.getState().replace('frostveil', 'crystal-lantern', 'lantern-rebuilt');

    expect(useBuiltStore.getState().replacing).toBeNull();
    expect(assets().some(a => a.id === 'lantern-rebuilt')).toBe(false);
    const lantern = assets().find(a => a.id === 'crystal-lantern');
    expect(lantern?.versions.map(v => v.version)).toEqual([1, 2, 3]);
    const item = useBuiltStore.getState().items.find(b => b.asset === 'crystal-lantern');
    expect(item?.state).toBe('current');
    expect(item?.source.replaced?.versions).toEqual([1, 2]);
    expect(useAppStore.getState().notice).toMatchObject({
      status: 'done',
      title: "Lantern rebuilt is in Crystal lantern's place",
    });

    await useBuiltStore.getState().undoReplace('frostveil', 'crystal-lantern');

    expect(assets().find(a => a.id === 'crystal-lantern')?.versions).toHaveLength(2);
    expect(assets().some(a => a.id === 'lantern-rebuilt')).toBe(true);
    expect(useBuiltStore.getState().items.map(b => b.asset)).toEqual([
      'laptop',
      'newtons-cradle',
      'lantern-rebuilt',
    ]);
    expect(useAppStore.getState().notice?.title).toBe('Crystal lantern is back on v2');
  });

  it('rejects with the reason and leaves the sheet open', async () => {
    await useBuiltStore.getState().load('frostveil');
    const target = { mode: 'replace' as const, asset: 'crystal-lantern', built: 'newtons-cradle' };
    useBuiltStore.setState({ replacing: target });
    await expect(
      useBuiltStore.getState().replace('frostveil', 'crystal-lantern', 'newtons-cradle')
    ).rejects.toMatchObject({
      message: expect.stringContaining('has a script that changed since its last build'),
    });
    expect(useBuiltStore.getState().replacing).toEqual(target);
    await expect(useBuiltStore.getState().undoReplace('frostveil', 'laptop')).rejects.toMatchObject(
      { message: expect.stringContaining('took no other model') }
    );
  });
});
