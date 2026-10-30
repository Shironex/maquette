import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAppStore } from './use-app-store';
import { useInboxStore } from './use-inbox-store';
import { useTriageStore } from './use-triage-store';

const ids = () => useInboxStore.getState().snapshot!.items.map(i => i.id);

beforeEach(async () => {
  await openMockWorkspace('configured', undefined, undefined, 'triage');
  await useInboxStore.getState().load();
  await useTriageStore.getState().loadDiscarded();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null, checked: [] });
  useTriageStore.setState({ layout: 'one', filter: 'all', discarded: [], working: false });
  useAppStore.setState({ notice: null });
});

describe('the triage store', () => {
  it('accepts several models into their assets and skips an unreadable one', async () => {
    useInboxStore.getState().check('frozen_angel.glb', true);
    const done = await useTriageStore
      .getState()
      .acceptMany(['frozen_angel.glb', 'observatory_arch_v4.glb', 'rune_obelisk.glb']);
    expect(done).toBe(true);
    expect(ids()).not.toContain('frozen_angel.glb');
    expect(ids()).not.toContain('observatory_arch_v4.glb');
    expect(useInboxStore.getState().checked).toEqual([]);
    const notice = useAppStore.getState().notice!;
    expect(notice.title).toBe('2 models accepted');
    expect(notice.detail).toMatch(/Rune obelisk/);
    const arch = useAppStore
      .getState()
      .workspace!.projects.find(p => p.id === 'frostveil')!
      .assets!.find(a => a.id === 'observatory-arch')!;
    expect(arch.versions.at(-1)?.version).toBe(4);
  });

  it('adds the accepted assets to the library when asked', async () => {
    await useTriageStore
      .getState()
      .acceptMany(['frozen_angel.glb'], { library: true, tags: ['frostveil'] });
    expect(useAppStore.getState().notice?.title).toBe('1 model accepted, 1 added to the library');
  });

  it('discards several, lists them first under discarded, and restores one aimed at its asset', async () => {
    expect(useTriageStore.getState().discarded).toHaveLength(3);
    await useTriageStore
      .getState()
      .discardMany(['crystal_lantern_redo.glb', 'observatory_arch_v3.glb']);
    expect(ids()).not.toContain('crystal_lantern_redo.glb');
    const discarded = useTriageStore.getState().discarded;
    expect(discarded).toHaveLength(5);
    expect(discarded.map(d => d.name)).toContain('crystal_lantern_redo.glb');
    expect(discarded.find(d => d.name === 'crystal_lantern_redo.glb')?.replacedBy).toBe(2);

    await useTriageStore.getState().restore('crystal_lantern_redo.glb');
    const back = useInboxStore
      .getState()
      .snapshot!.items.find(i => i.id === 'crystal_lantern_redo.glb')!;
    expect(back.link).toBeNull();
    expect(back.aim).toMatchObject({ project: 'frostveil', asset: 'crystal-lantern' });
    expect(useInboxStore.getState().selected).toBe('crystal_lantern_redo.glb');
    expect(useTriageStore.getState().discarded).toHaveLength(4);
  });
});
