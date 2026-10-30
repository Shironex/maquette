import { useState } from 'react';
import { boneGroups, namingLabel } from '@/state/avatar';
import { useAvatarStore } from '@/state/use-avatar-store';

/**
 * The bone groups, which optional ones the person opened, the bone whose
 * joint list is open, and what the name matching made of the rig.
 */
export function useBoneMap() {
  const readiness = useAvatarStore(s => s.view?.readiness ?? null);
  const overrides = useAvatarStore(s => s.view?.spec.bones ?? {});
  const savingBone = useAvatarStore(s => s.savingBone);
  const [open, setOpen] = useState<string[]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const naming = readiness?.naming ?? null;
  return {
    readiness,
    overrides,
    savingBone,
    groups: readiness
      ? boneGroups(readiness).map(g => ({
          ...g,
          shown: g.id === 'required' || open.includes(g.id),
        }))
      : [],
    missing: readiness?.bones.filter(b => b.required && !b.joint).length ?? 0,
    convention: naming?.convention ? namingLabel(naming.convention) : null,
    unclear: naming ? !naming.confident : false,
    editing,
    edit: (bone: string | null) => setEditing(e => (e === bone ? null : bone)),
    toggle: (id: string) => setOpen(o => (o.includes(id) ? o.filter(x => x !== id) : [...o, id])),
  };
}
