import { useEffect, useRef, type PointerEvent } from 'react';
import { frameAt, n, readout, ticks, timeOfFrame, type Speed } from '@/state/motion';
import { currentClip, useMotionStore } from '@/state/use-motion-store';

/** True when a key press is meant for a text field, not the player. */
function typing(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return Boolean(target.closest('input, textarea, select, [contenteditable="true"]'));
}

/** The player's state for the selected clip, its controls and its keys. */
export function useClipTimeline() {
  const clip = useMotionStore(s => currentClip(s));
  const player = useMotionStore(s => s.player);
  const setPlayer = useMotionStore(s => s.setPlayer);
  const frames = clip?.frames ?? 0;
  const [from, to] = player.range;
  const frame = clip ? frameAt(clip, player.time) : 0;
  const seek = (f: number) => {
    if (!clip) return;
    const at = Math.min(Math.max(Math.round(f), 0), frames);
    setPlayer({ time: timeOfFrame(clip, at), playing: false });
  };
  const controls = {
    toggle: () => {
      if (!clip) return;
      const atEnd = frame >= to && !player.loop;
      setPlayer({
        playing: !player.playing,
        ...(atEnd ? { time: timeOfFrame(clip, from) } : {}),
      });
    },
    toStart: () => seek(from),
    toEnd: () => seek(to),
    step: (by: number) => seek(frame + by),
    setIn: () => setPlayer({ range: [Math.min(frame, to - 1 < 0 ? 0 : to - 1), to] }),
    setOut: () => setPlayer({ range: [from, Math.max(frame, from + 1)] }),
    setLoop: (loop: boolean) => setPlayer({ loop }),
    setSpeed: (speed: Speed) => setPlayer({ speed }),
    setRange: (range: [number, number]) => setPlayer({ range }),
    seek,
  };
  const latest = useRef(controls);
  latest.current = controls;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
      const c = latest.current;
      const keys: Record<string, () => void> = {
        ' ': c.toggle,
        ',': () => c.step(-1),
        '.': () => c.step(1),
        i: c.setIn,
        o: c.setOut,
        l: () => c.setLoop(!useMotionStore.getState().player.loop),
      };
      const run = keys[e.key.toLowerCase()];
      if (!run) return;
      e.preventDefault();
      run();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  return {
    clip,
    player,
    frame,
    frames,
    readout: clip ? readout(clip, player.time) : '',
    ticks: ticks(frames),
    fps: n(clip?.fps),
    controls,
  };
}

/** The frame under a pointer on a lane `frames` long. */
export function frameUnder(e: PointerEvent<HTMLElement>, frames: number): number {
  const box = e.currentTarget.getBoundingClientRect();
  const share = box.width > 0 ? (e.clientX - box.left) / box.width : 0;
  return Math.round(Math.min(Math.max(share, 0), 1) * frames);
}
