import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { useAnimationsStore } from './use-animations-store';

afterEach(() => {
  clearMocks();
  useAnimationsStore.setState({ view: null, importing: null, saving: null, selected: null });
});

const DROP = '/Users/me/Downloads';

describe('the animation library store', () => {
  it('reads files, keeps a duplicate only when asked, and imports the rest', async () => {
    await openMockLibrary('demo');
    const store = useAnimationsStore.getState();
    await store.load();
    const before = useAnimationsStore.getState().view!.items.length;
    store.openImport();
    await store.addFiles([`${DROP}/Pro Sword and Shield Pack.zip`]);
    await store.addFiles([`${DROP}/heroine_cast_take3.fbx`, `${DROP}/notes.txt`]);
    const sheet = useAnimationsStore.getState().importing!;
    expect(sheet.paths).toHaveLength(3);
    const [zip, custom, text] = sheet.plan!.files;
    expect(zip!.clips.find(c => c.name === 'Sword And Shield Idle')?.status).toBe('duplicate');
    expect(custom!.rig).toBe('custom');
    expect(text!.leftOut[0]!.reason).toMatch(/not an FBX/);
    store.changeImport({ choices: { keepBoth: [zip!.clips[0]!.key], skip: [] } });
    await useAnimationsStore.getState().runImport();
    const after = useAnimationsStore.getState();
    expect(after.importing).toBeNull();
    expect(after.view!.items.length).toBe(before + 5);
    expect(after.view!.items.filter(i => i.clip.name === 'Sword And Shield Idle')).toHaveLength(2);
  });

  it('saves a rigged model clips as a pack, and skips them the second time', async () => {
    await openMockLibrary('demo');
    const store = useAnimationsStore.getState();
    await store.load();
    await store.openSave({ project: null, asset: null, inbox: 'elsheria.glb' });
    const sheet = useAnimationsStore.getState().saving!;
    expect(sheet.plan!.rig).toBe('mixamo');
    expect(sheet.pack).toBe('elsheria moves');
    expect(sheet.chosen.length).toBeGreaterThan(0);
    await useAnimationsStore.getState().runSave();
    const saved = useAnimationsStore
      .getState()
      .view!.items.filter(i => i.clip.pack === 'elsheria moves');
    expect(saved).toHaveLength(sheet.chosen.length);
    expect(saved[0]!.clip.origin.source).toBe('tripo');
  });

  it('says why a model without a rig has nothing to save', async () => {
    await openMockLibrary();
    await useAnimationsStore
      .getState()
      .openSave({ project: null, asset: null, inbox: 'veil_gate.glb' });
    expect(useAnimationsStore.getState().saving!.error).toMatch(/no rig/);
  });
});
