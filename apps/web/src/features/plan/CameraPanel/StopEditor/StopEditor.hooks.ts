import { useEffect, useState } from 'react';
import type { LayoutStop } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { moved } from '@/state/plan-camera';
import { usePlanCameraStore } from '@/state/use-plan-camera-store';
import { usePlansStore } from '@/state/use-plans-store';

/** Metres kept to the millimetre and degrees to a tenth, so a value reads back as typed. */
const round = (n: number, places: number) => Math.round(n * 10 ** places) / 10 ** places;

/**
 * The selected stop's settings: its name, where the camera stands and
 * looks, its field of view, and its place in the main line. Each change is
 * an edit of the canvas, saved like any other and undoable.
 */
export function useStopEditor(nodeId: string) {
  const node = usePlansStore(s => s.nodes.find(n => n.id === nodeId)?.data.node ?? null);
  const updateNode = usePlansStore(s => s.updateNode);
  const replaceFlow = usePlansStore(s => s.replaceFlow);
  const onNodesChange = usePlansStore(s => s.onNodesChange);
  const main = usePlanCameraStore(s => s.view?.main ?? []);
  const stop = node?.stop ?? null;
  const at = main.indexOf(nodeId);
  // The name commits on Enter or leaving the field, and never empty.
  const [name, setName] = useState(stop?.name ?? '');
  useEffect(() => setName(stop?.name ?? ''), [stop?.name]);
  const set = (fields: Partial<LayoutStop>) => {
    if (stop) updateNode(nodeId, { stop: { ...stop, ...fields } });
  };
  const axis = (key: 'position' | 'target', i: number, value: number) => {
    if (!stop) return;
    const next = [...stop[key]] as LayoutStop['position'];
    next[i] = round(value, 3);
    set({ [key]: next });
  };
  const move = (delta: -1 | 1) => {
    const { nodes, edges } = usePlansStore.getState();
    const next = moved(nodes, edges, main, nodeId, delta, clock.nowMs());
    if (next) replaceFlow(next.nodes, next.edges);
  };
  return {
    stop,
    by: node?.by ?? null,
    place: at >= 0 ? at + 1 : null,
    count: main.length,
    name,
    setName,
    commitName: () => {
      const next = name.trim().slice(0, 120);
      if (next && next !== stop?.name) set({ name: next });
      else setName(stop?.name ?? '');
    },
    axis,
    fov: (value: number) => set({ fov: round(value, 1) }),
    earlier: () => move(-1),
    later: () => move(1),
    remove: () => onNodesChange([{ type: 'remove', id: nodeId }]),
  };
}
