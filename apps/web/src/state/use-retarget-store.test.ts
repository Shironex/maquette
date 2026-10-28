import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { useAnimationsStore } from './use-animations-store';
import { useAppStore } from './use-app-store';
import { useRetargetStore } from './use-retarget-store';

afterEach(() => {
  clearMocks();
  useRetargetStore.setState({ sheet: null, busy: false });
  useAnimationsStore.setState({ view: null });
});

const CLIPS = ['sword-and-shield-slash', 'sword-and-shield-run', 'sword-and-shield-crouch'];

describe('the retarget store', () => {
  it('walks the steps, bakes a draft and adds the good clips as the next version', async () => {
    await openMockLibrary('demo');
    const store = useRetargetStore.getState();
    await store.open({ project: 'frostveil', asset: 'elsheria' });
    for (const clip of CLIPS) useRetargetStore.getState().toggle(clip);
    await useRetargetStore.getState().next();
    let sheet = useRetargetStore.getState().sheet!;
    expect(sheet.plan!.needsLook).toBe(false);
    expect(sheet.step).toBe('settings');
    expect(sheet.settings.map(s => s.name)).toEqual(['sword_slash', 'sword_run', 'sword_crouch']);

    useRetargetStore.getState().changeSetting('sword-and-shield-slash', { name: 'slash_one' });
    useRetargetStore.getState().resetNames();
    expect(useRetargetStore.getState().sheet!.settings[0]!.name).toBe('sword_slash');

    await useRetargetStore.getState().next();
    sheet = useRetargetStore.getState().sheet!;
    expect(sheet.step).toBe('review');
    expect(sheet.draft!.review.clips.map(c => c.status)).toEqual(['good', 'look', 'failed']);

    const before = useAppStore
      .getState()
      .workspace!.project!.assets!.find(a => a.id === 'elsheria')!;
    await useRetargetStore.getState().bake();
    expect(useRetargetStore.getState().sheet).toBeNull();
    const after = useAppStore
      .getState()
      .workspace!.project!.assets!.find(a => a.id === 'elsheria')!;
    expect(after.versions).toHaveLength(before.versions.length + 1);
  });

  it('reuses a clip baked before with its settings and its name', async () => {
    await openMockLibrary('demo');
    await useRetargetStore.getState().open({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['standing-dodge-left'],
      step: 'settings',
    });
    const plan = useRetargetStore.getState().sheet!.plan!;
    expect(plan.outdated).toEqual(['standing-dodge-left']);
    expect(plan.settings[0]!.name).toBe('dodge_left');
  });

  it('maps a bone by hand and keeps the choice in the plan', async () => {
    await openMockLibrary('demo');
    await useRetargetStore
      .getState()
      .open({ project: 'frostveil', asset: 'elsheria', step: 'map' });
    await useRetargetStore.getState().pick('Neck', null);
    const neck = useRetargetStore.getState().sheet!.plan!.map.find(p => p.bone === 'Neck')!;
    expect(neck).toMatchObject({ target: null, how: 'missing' });
    expect(useRetargetStore.getState().sheet!.picked).toEqual([{ bone: 'Neck', target: null }]);
  });
});
