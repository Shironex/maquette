import { useEffect, useState } from 'react';
import { commands, errorMessage, type GripHands, type LayerSettings } from '@/ipc/client';
import { boneSide, handsOfPose, otherHand, poseFor, updated } from '@/state/recipe-grips';
import { gripNote } from '@/state/recipe-library';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';

/**
 * A grip change's own settings: the hands its copy poses, the blank it
 * holds, its eases, the library grip it came from and the offer to take a
 * newer revision of it.
 */
export function useRecipeGripForm(id: string) {
  const draft = useRecipeEditorStore(s => s.editing?.draft ?? null);
  const list = useGripLibraryStore(s => s.list);
  const [updating, setUpdating] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  useEffect(() => {
    if (!useGripLibraryStore.getState().list) void useGripLibraryStore.getState().load();
  }, []);
  const layer = draft?.layers.find(l => l.id === id && l.kind === 'grip') ?? null;
  if (!draft || !layer) return null;
  const { edit } = useRecipeEditorStore.getState();
  const put = (next: typeof layer) =>
    edit(d => ({ ...d, layers: d.layers.map(l => (l.id === id ? next : l)) }));
  const settle = (patch: Partial<LayerSettings>) =>
    put({ ...layer, settings: { ...layer.settings, ...patch } });
  const pose = layer.pose ?? {};
  const holds = boneSide(layer.bone) ?? 'right';
  const hands = handsOfPose(pose) ?? holds;
  const from = layer.from ?? null;
  const entry = from ? list?.find(g => g.id === from.entry) : null;
  const note = gripNote(from, list);
  const update = async () => {
    if (note?.kind !== 'newer' || !from) return;
    setUpdating(true);
    setRefused(null);
    try {
      const doc = await commands.gripLibraryGet(note.entry, note.latest);
      const link = { entry: note.entry, uid: doc.entry.uid, revision: doc.revision };
      // Read again: the person may have edited the change while the grip was read.
      const now = useRecipeEditorStore.getState().editing?.draft.layers.find(l => l.id === id);
      if (now) put(updated(now, doc.grip, link));
    } catch (error) {
      setRefused(errorMessage(error));
    } finally {
      setUpdating(false);
    }
  };
  const blanks = draft.blanks;
  return {
    layer,
    /** "Scythe, staff grip r1", or that it has no library link. */
    title: from ? `${entry?.name ?? from.entry} r${from.revision}` : 'Its own pose',
    hands,
    setHands: (next: GripHands) => {
      const supports = handsOfPose(pose) === 'both' ? otherHand(holds) : null;
      const taken = poseFor({ hands: { holds, supports }, pose }, next);
      put({ ...layer, bone: `${taken.holds}Hand`, pose: taken.pose });
    },
    bones: Object.keys(pose).length,
    target: layer.settings.target ?? '',
    targets: [
      { value: '', label: 'Nothing, the hands only' },
      ...blanks.map(b => ({ value: b.id, label: `${b.id}: ${b.label}` })),
    ],
    setTarget: (target: string) => settle({ target: target || null }),
    easeIn: layer.settings.easeIn ?? 0,
    easeOut: layer.settings.easeOut ?? 0,
    setEaseIn: (v: number) => settle({ easeIn: Math.max(0, Math.round(v)) }),
    setEaseOut: (v: number) => settle({ easeOut: Math.max(0, Math.round(v)) }),
    note,
    current: from?.revision ?? null,
    updating,
    refused,
    update: () => void update(),
  };
}
