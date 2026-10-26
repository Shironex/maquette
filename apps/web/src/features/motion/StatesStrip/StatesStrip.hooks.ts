import { n } from '@/state/motion';
import { MAX_STATES, addState, transitionInto } from '@/state/motion-states';
import { useMotionStore } from '@/state/use-motion-store';

const EASE_WORD = {
  linear: 'linear',
  'ease-in': 'ease in',
  'ease-out': 'ease out',
  'ease-in-out': 'ease in out',
} as const;

/** The states of the selected model in order, with the move into each. */
export function useStatesStrip() {
  const asset = useMotionStore(s => s.asset);
  const object = useMotionStore(s => (s.asset ? s.draft.states[s.asset] : undefined));
  const selected = useMotionStore(s => s.state);
  const selectState = useMotionStore(s => s.selectState);
  const editStates = useMotionStore(s => s.editStates);
  if (!object || !asset) return null;
  const cards = object.states.map((state, i) => {
    const into = i > 0 ? transitionInto(object, state.id, object.states[i - 1]!.id) : null;
    const t = into?.transition;
    const moved = Object.keys(state.parts).length;
    return {
      id: state.id,
      name: state.name,
      line: i === 0 ? 'rest pose' : `${moved} ${moved === 1 ? 'part' : 'parts'} moved`,
      into: t
        ? { time: `${n(t.duration)} s`, word: n(t.stagger) > 0 ? 'stagger' : EASE_WORD[t.easing] }
        : null,
      current: state.id === (selected ?? object.states[0]?.id),
    };
  });
  return {
    cards,
    selectState,
    full: object.states.length >= MAX_STATES,
    add: () => {
      const name = `State ${object.states.length + 1}`;
      editStates(asset, o => addState(o, name, selected ?? undefined));
      const next = useMotionStore.getState().draft.states[asset]?.states.at(-1);
      if (next) selectState(next.id);
    },
  };
}
