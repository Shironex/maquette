import { useEffect, useState } from 'react';
import { fileUrl } from '@/ipc/files';
import type { PlanNode } from '@/ipc/client';
import { usePlansStore } from '@/state/use-plans-store';

/** Which text a node is editing: a note's text, or a picture's or link's title. */
export type Editing = 'text' | 'title' | null;

/**
 * One node's picture URL and its inline edit: a double click starts it,
 * leaving the field or Mod+Enter keeps it, Escape drops it.
 */
export function usePlanNodeCard(node: PlanNode) {
  const updateNode = usePlansStore(s => s.updateNode);
  const openLink = usePlansStore(s => s.openLink);
  const file = usePlansStore(s => (node.image ? (s.images[node.image] ?? null) : null));
  // A note just made opens for writing.
  const [editing, setEditing] = useState<Editing>(() =>
    usePlansStore.getState().fresh === node.id ? 'text' : null
  );
  const [value, setValue] = useState('');
  const start = (what: Exclude<Editing, null>) => {
    setValue(what === 'text' ? node.text : node.title);
    setEditing(what);
  };
  useEffect(() => {
    const store = usePlansStore.getState();
    if (store.fresh === node.id) store.clearFresh();
  }, [node.id]);
  const commit = () => {
    if (editing === 'text' && value !== node.text) updateNode(node.id, { text: value });
    if (editing === 'title' && value.trim() !== node.title)
      updateNode(node.id, { title: value.trim() });
    setEditing(null);
  };
  return {
    src: fileUrl(file),
    editing,
    value,
    setValue,
    start,
    commit,
    cancel: () => setEditing(null),
    openLink: () => void openLink(node.id),
    canOpen: node.url?.startsWith('https://') ?? false,
    copyLink: () => {
      if (node.url) void navigator.clipboard?.writeText(node.url);
    },
  };
}
