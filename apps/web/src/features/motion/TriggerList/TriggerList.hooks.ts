import { useState } from 'react';
import type { TriggerKind } from '@/ipc/client';
import { addTrigger, newTrigger, removeTrigger, triggerText } from '@/state/motion-states';
import { useMotionStore } from '@/state/use-motion-store';

export const TRIGGER_ICON = {
  stop: 'video-camera',
  scroll: 'mouse-scroll',
  click: 'cursor-click',
  hover: 'hand-pointing',
} as const;

/** The triggers of the selected model's states, and adding one for the selected state. */
export function useTriggerList() {
  const asset = useMotionStore(s => s.asset);
  const object = useMotionStore(s => (s.asset ? s.draft.states[s.asset] : undefined));
  const stateId = useMotionStore(s => s.state);
  const stops = useMotionStore(s => s.view?.stops ?? []);
  const editStates = useMotionStore(s => s.editStates);
  const [menu, setMenu] = useState(false);
  if (!object || !asset) return null;
  const target =
    object.states.find(s => s.id === stateId && s.id !== object.states[0]?.id) ?? object.states[1];
  const rows = object.triggers.map((trigger, i) => ({
    key: `${trigger.kind}-${trigger.to}-${i}`,
    icon: TRIGGER_ICON[trigger.kind],
    ...triggerText(object, trigger, stops),
    remove: () => editStates(asset, o => removeTrigger(o, i)),
  }));
  return {
    rows,
    menu,
    setMenu,
    target: target?.name ?? null,
    hasStops: stops.length > 0,
    add: (kind: TriggerKind) =>
      target && editStates(asset, o => addTrigger(o, newTrigger(kind, target.id, stops))),
  };
}
