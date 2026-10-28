import { useEffect, type RefObject } from 'react';
import { isTopModal, useModalFocus } from './useModalFocus';

/**
 * Calls `onClose` on Escape while `ref` is the topmost modal dialog, so a
 * dialog over a sheet closes alone, and an open menu (portalled to the body)
 * closes before the dialog does.
 * Also focuses its `[data-close]` button, keeps Tab inside it and gives
 * focus back to the opener on close (see `useModalFocus`).
 */
export function useTopEscape(ref: RefObject<HTMLElement | null>, onClose: () => void) {
  useModalFocus(ref);
  useEffect(() => {
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
