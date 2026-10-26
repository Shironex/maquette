import type { StateTransition } from '@/ipc/client';
import { n } from '@/state/motion';
import { ease, partsInMove, setTransition, transitionInto } from '@/state/motion-states';
import { useMotionStore } from '@/state/use-motion-store';

/** An SVG path of the easing curve across a `w` by `h` box. */
export function curvePath(easing: StateTransition['easing'], w: number, h: number): string {
  const points = Array.from({ length: 25 }, (_, i) => {
    const t = i / 24;
    return `${(t * w).toFixed(1)},${(h - ease(easing, t) * h).toFixed(1)}`;
  });
  return `M${points.join(' L')}`;
}

/** The move into the selected state: its timing, easing and the parts it moves. */
export function useTransitionEditor() {
  const asset = useMotionStore(s => s.asset);
  const object = useMotionStore(s => (s.asset ? s.draft.states[s.asset] : undefined));
  const stateId = useMotionStore(s => s.state);
  const parts = useMotionStore(
    s => s.view?.models.find(m => m.asset === s.asset)?.parts.length ?? 0
  );
  const editStates = useMotionStore(s => s.editStates);
  if (!object || !asset || !stateId) return null;
  const index = object.states.findIndex(s => s.id === stateId);
  if (index <= 0) return null;
  const into = transitionInto(object, stateId, object.states[index - 1]!.id);
  if (!into) return null;
  const { transition: t } = into;
  const name = (id: string) => object.states.find(s => s.id === id)?.name ?? id;
  const moving = partsInMove(object, t);
  const patch = (p: Parameters<typeof setTransition>[2]) =>
    editStates(asset, o => setTransition(o, into.index, p));
  return {
    title: `${name(t.from)} to ${name(t.to)}`,
    duration: n(t.duration),
    delay: n(t.delay),
    stagger: n(t.stagger),
    easing: t.easing,
    moving:
      moving.length > 0
        ? `${moving.slice(0, 2).join(', ')}${moving.length > 2 ? ` and ${moving.length - 2} more` : ''}`
        : 'none yet',
    of: `${moving.length} of ${parts}`,
    patch,
  };
}
