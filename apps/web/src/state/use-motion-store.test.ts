import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMotion } from '@/test/open-motion';
import { useMotionDirty, useMotionStore } from './use-motion-store';
import { renderHook } from '@testing-library/react';

afterEach(() => clearMocks());

describe('the motion store', () => {
  it('reads the demo models and selects the first that moves', async () => {
    const s = await openMotion();
    const names = s.view?.models.map(m => m.asset) ?? [];
    expect(names).toEqual(expect.arrayContaining(['clay-figure', 'switch-cabinet', 'turntable']));
    expect(s.asset).toBe('clay-figure');
    expect(s.tab).toBe('clips');
    expect(s.clip).toBe('Idle breathe');
    expect(s.player.range).toEqual([
      0,
      s.view!.models.find(m => m.asset === 'clay-figure')!.clips[0]!.frames,
    ]);
  });

  it('keeps a clip off the site and saves it into the layout', async () => {
    await openMotion();
    useMotionStore.getState().setShip('clay-figure', 'Wave', false);
    const { result } = renderHook(() => useMotionDirty());
    expect(result.current).toBe(true);
    expect(await useMotionStore.getState().save()).toBe(true);
    const s = useMotionStore.getState();
    expect(s.view?.clipsOff['clay-figure']).toEqual(['Wave']);
    expect(s.draft).toEqual(s.saved);
  });

  it('gives a still model its rest state and opens States', async () => {
    await openMotion();
    useMotionStore.getState().giveStates('turntable');
    const s = useMotionStore.getState();
    expect(s.tab).toBe('states');
    expect(s.draft.states.turntable?.states.map(x => x.name)).toEqual(['Rest']);
    expect(s.state).toBe('rest');
    useMotionStore.getState().revert();
    expect(useMotionStore.getState().draft.states.turntable).toBeUndefined();
  });

  it('holds the draft when the layout changes under it', async () => {
    await openMotion();
    useMotionStore.getState().setShip('clay-figure', 'Wave', false);
    await useMotionStore.getState().onChanged('frostveil');
    expect(useMotionStore.getState().outside).toBe(true);
  });

  it('moves the playhead on and round the loop', async () => {
    await openMotion();
    const clip = useMotionStore.getState().view!.models[0]!.clips[0]!;
    useMotionStore.getState().setPlayer({ time: 0, playing: true, loop: true });
    useMotionStore.getState().tick((clip.duration ?? 1) + 0.1);
    const { player } = useMotionStore.getState();
    expect(player.playing).toBe(true);
    expect(player.time).toBeLessThan(clip.duration ?? 1);
  });
});
