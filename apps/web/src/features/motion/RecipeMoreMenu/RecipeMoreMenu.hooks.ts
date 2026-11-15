import { useState } from 'react';
import type { MenuItem } from '@/features/common/Menu';

/** Whether a recipe's More menu is open. */
export function useMoreMenu() {
  const [open, setOpen] = useState(false);
  return { open, toggle: () => setOpen(o => !o), close: () => setOpen(false) };
}

/** Why Remove recipe is greyed out now, or null when it can go. */
export function removeBlocked(running: boolean, dirty = false): string | null {
  if (running) return 'It is baking. Wait for the bake or cancel it first.';
  if (dirty) return 'Save or discard your edits first.';
  return null;
}

/** Remove recipe, into the workspace trash with Undo on the notice. */
export function removeItem(onRemove: () => void, blocked: string | null): MenuItem {
  return {
    id: 'remove',
    label: 'Remove recipe',
    icon: 'trash',
    tone: 'danger',
    detail: 'To the workspace trash, with Undo',
    disabled: blocked !== null,
    hint: blocked ?? undefined,
    onSelect: onRemove,
  };
}
