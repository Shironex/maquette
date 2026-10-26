import { useEffect, useRef, useState } from 'react';
import { fileUrl } from '@/ipc/files';
import { n } from '@/state/motion';
import { partsInMove, posesAt, posesOf, transitionInto } from '@/state/motion-states';
import { useMotionStore } from '@/state/use-motion-store';

/**
 * Plays `progress` from 0 to 1 over `seconds` while `playing`, then stops.
 * The move preview: how the site would run it once a trigger fires.
 */
export function useMovePlayer(seconds: number) {
  const [progress, setProgress] = useState(1);
  const [playing, setPlaying] = useState(false);
  const start = useRef(0);
  useEffect(() => {
    if (!playing) return;
    let frame = 0;
    start.current = performance.now();
    const step = (now: number) => {
      const p = Math.min(1, (now - start.current) / 1000 / Math.max(seconds, 0.05));
      setProgress(p);
      if (p < 1) frame = requestAnimationFrame(step);
      else setPlaying(false);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [playing, seconds]);
  return {
    progress,
    playing,
    scrub: (p: number) => {
      setPlaying(false);
      setProgress(p);
    },
    play: () => {
      setProgress(0);
      setPlaying(true);
    },
  };
}

/** The selected model in its selected state, or partway through the move into it. */
export function useStatesStage() {
  const view = useMotionStore(s => s.view);
  const asset = useMotionStore(s => s.asset);
  const stateId = useMotionStore(s => s.state);
  const object = useMotionStore(s => (s.asset ? s.draft.states[s.asset] : undefined));
  const selectState = useMotionStore(s => s.selectState);
  const giveStates = useMotionStore(s => s.giveStates);
  const [showParts, setShowParts] = useState(true);
  const [failed, setFailed] = useState<{ url: string | null; message: string } | null>(null);
  const model = view?.models.find(m => m.asset === asset) ?? null;
  const state = object?.states.find(s => s.id === stateId) ?? object?.states[0] ?? null;
  const into = object && state ? transitionInto(object, state.id) : null;
  const t = into?.transition ?? null;
  const seconds = t
    ? n(t.duration) + n(t.stagger) * Math.max(0, partsInMove(object!, t).length - 1)
    : 0;
  const player = useMovePlayer(seconds);
  const poses =
    object && state ? (t ? posesAt(object, t, player.progress) : posesOf(object, state.id)) : null;
  const moved = object && t ? partsInMove(object, t) : Object.keys(state?.parts ?? {});
  const url = model ? fileUrl(model.file) : null;
  const pct = Math.round(player.progress * 100);
  const badge = state
    ? t
      ? `${state.name} · ${pct === 100 ? 'end' : `${pct}%`} of the ${n(t.duration)} s move`
      : `${state.name} · ${state === object?.states[0] ? 'rest pose' : 'no move into it'}`
    : null;
  return {
    model,
    object,
    state,
    transition: t,
    selectState,
    give: () => model && giveStates(model.asset),
    poses,
    outline: showParts ? moved : [],
    showParts,
    setShowParts,
    player,
    badge,
    url,
    label: model ? `${model.name}${state ? ` in ${state.name}` : ''}` : 'No model',
    failed: (failed?.url === url ? failed.message : null) ?? model?.error ?? null,
    setFailed: (message: string) => setFailed({ url, message }),
  };
}
