import { useEffect, type RefObject } from 'react';

/**
 * Calls `onClose` on Escape while `ref` is the topmost modal dialog, so a
 * dialog over a sheet closes alone. Also focuses its `[data-close]` button.
 */
export function useTopEscape(ref: RefObject<HTMLElement | null>, onClose: () => void) {
  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>('[data-close]')?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || !ref.current) return;
      const modals = document.querySelectorAll('[aria-modal="true"]');
      if (modals[modals.length - 1] !== ref.current) return;
      onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [ref, onClose]);
}
