import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary, openMockWorkspace } from '@/test/mock-ipc';
import { assetQuickView, isViewable, libraryQuickView, quickViewLine } from './quick-view';
import { useAppStore } from './use-app-store';

afterEach(() => clearMocks());

const workspace = () => useAppStore.getState().workspace!;
const asset = (id: string) => workspace().project!.assets!.find(a => a.id === id)!;

describe('the 3D quick view targets', () => {
  it('shows a card at its newest version, ready for the Inspector', async () => {
    await openMockWorkspace();
    const target = assetQuickView(asset('crystal-lantern'), workspace());
    expect(target).toMatchObject({
      name: 'Crystal lantern',
      version: 2,
      from: 'Frostveil',
      inspect: { project: 'frostveil', asset: 'crystal-lantern', version: 2 },
    });
    expect(target!.file).toMatch(/\/projects\/frostveil\/models\/crystal_lantern_v2\.glb$/);
    expect(quickViewLine(target!)).toBe('v2 · 12.9k triangles · 400 KB · Frostveil');
    expect(assetQuickView({ ...asset('crystal-lantern'), versions: [] }, workspace())).toBeNull();
  });

  it("opens a library model's copy in the open project, and says why not without one", async () => {
    await openMockLibrary('demo');
    const { items } = await commands.libraryGet();
    const item = (id: string) => items.find(i => i.entry.id === id)!;
    expect(libraryQuickView(item('elsheria'), workspace())).toMatchObject({
      version: 3,
      file: `${item('elsheria').dir}/models/elsheria_v3.glb`,
      from: 'Library',
      inspect: { project: 'frostveil', asset: 'elsheria', version: 2 },
    });
    expect(libraryQuickView(item('rune-obelisk'), workspace())).toMatchObject({
      inspect: null,
      inspectBlocked: 'Add it to Frostveil to inspect it in full',
    });
    expect(libraryQuickView(item('crypt-brazier'), workspace())).toBeNull();
  });

  it('reads GLB and glTF files only', () => {
    expect(isViewable('models/a_v1.glb')).toBe(true);
    expect(isViewable('models/a_v1.GLTF')).toBe(true);
    expect(isViewable('models/a_v1/a.obj')).toBe(false);
  });
});
