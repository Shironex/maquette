import { useEffect, type RefObject } from 'react';
import { isTopModal, useModalFocus } from './useModalFocus';

/**
 * Calls `onClose` on Escape while `ref` is the topmost modal dialog, so a
 * dialog over a sheet closes alone, and an open menu (portalled to the body)
 * closes before the dialog does.
 * Also focuses its `[data-close]` button, keeps Tab inside it and gives
 * focus back to the opener on close (see `useModalFocus`).
 * The layer itself can hold focus (tabindex -1), so a press on something that
 * takes none, such as a mode button in WebKit or the 3D canvas, leaves focus
 * inside it rather than on the page behind, where Escape no longer reaches it.
 */
export function useTopEscape(ref: RefObject<HTMLElement | null>, onClose: () => void) {
  useModalFocus(ref);
  useEffect(() => {
    if (ref.current && !ref.current.hasAttribute('tabindex')) ref.current.tabIndex = -1;
    ref.current?.querySelector<HTMLButtonElement>('[data-close]')?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || !ref.current || !isTopModal(ref.current)) return;
      if (document.querySelector('[role="menu"]')) return;
      onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [ref, onClose]);
}
