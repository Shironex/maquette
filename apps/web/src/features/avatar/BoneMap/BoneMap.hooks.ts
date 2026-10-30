import { useState } from 'react';
import { boneGroups } from '@/state/avatar';
import { useAvatarStore } from '@/state/use-avatar-store';

/** The bone groups and which optional ones the person opened. */
export function useBoneMap() {
  const readiness = useAvatarStore(s => s.view?.readiness ?? null);
  const [open, setOpen] = useState<string[]>([]);
  return {
    readiness,
    groups: readiness
      ? boneGroups(readiness).map(g => ({
          ...g,
          shown: g.id === 'required' || open.includes(g.id),
        }))
      : [],
    missing: readiness?.bones.filter(b => b.required && !b.joint).length ?? 0,
    toggle: (id: string) => setOpen(o => (o.includes(id) ? o.filter(x => x !== id) : [...o, id])),
  };
}
