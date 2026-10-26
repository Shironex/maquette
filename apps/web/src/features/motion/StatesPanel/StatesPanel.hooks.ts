import { useMotionStore } from '@/state/use-motion-store';

/** Where the selected model's states are kept and what the site does with them. */
export function useStatesPanel() {
  const object = useMotionStore(s => (s.asset ? s.draft.states[s.asset] : undefined));
  const stops = useMotionStore(s => s.view?.stops ?? []);
  if (!object) return null;
  const at = [
    ...new Set(
      object.triggers
        .filter(t => t.kind === 'stop')
        .map(t => stops.findIndex(s => s.id === t.stop) + 1)
        .filter(i => i > 0)
    ),
  ].sort((a, b) => a - b);
  return {
    items: [
      { label: 'Saved in', value: 'layout.json', mono: true },
      { label: 'States', value: String(object.states.length), mono: true },
      {
        label: 'Camera stops',
        value: at.length > 0 ? at.join(', ') : 'none',
        mono: true,
      },
      { label: 'Plays on the site', value: 'Not yet' },
    ],
  };
}
