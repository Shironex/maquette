import { useState } from 'react';
import { poseOf, removeState, renameState, setPose, type Vec3 } from '@/state/motion-states';
import { useMotionStore } from '@/state/use-motion-store';

/** The selected state's name and how one of its parts is posed. */
export function usePoseEditor() {
  const asset = useMotionStore(s => s.asset);
  const object = useMotionStore(s => (s.asset ? s.draft.states[s.asset] : undefined));
  const stateId = useMotionStore(s => s.state);
  const parts = useMotionStore(s => s.view?.models.find(m => m.asset === s.asset)?.parts ?? []);
  const editStates = useMotionStore(s => s.editStates);
  const [picked, setPart] = useState<string | null>(null);
  if (!object || !asset) return null;
  const state = object.states.find(s => s.id === stateId) ?? object.states[0];
  if (!state) return null;
  const rest = state.id === object.states[0]?.id;
  const posed = Object.keys(state.parts);
  const part = picked && parts.includes(picked) ? picked : (posed[0] ?? parts[0] ?? null);
  const pose = poseOf(part ? state.parts[part] : undefined);
  const set = (key: 'position' | 'rotation', axis: number, value: number) => {
    if (!part) return;
    const next = { position: [...pose.position] as Vec3, rotation: [...pose.rotation] as Vec3 };
    next[key][axis] = value;
    editStates(asset, o => setPose(o, state.id, part, next));
  };
  return {
    state,
    rest,
    parts,
    posed,
    part,
    setPart,
    pose,
    set,
    rename: (name: string) =>
      name.trim() && editStates(asset, o => renameState(o, state.id, name.trim())),
    remove: () => editStates(asset, o => removeState(o, state.id)),
  };
}
