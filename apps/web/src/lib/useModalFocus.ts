import { useEffect, type RefObject } from 'react';

const TABBABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** The controls inside `root` that Tab reaches, in order. */
export function tabbables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(TABBABLE)].filter(
    el => el.tabIndex >= 0 && (el.checkVisibility?.() ?? true)
  );
}

/** True when `el` is the last open modal, the one on top. */
export function isTopModal(el: HTMLElement): boolean {
  const modals = document.querySelectorAll('[aria-modal="true"]');
  return modals[modals.length - 1] === el;
}

/** What had focus before the element that has it now, for a layer that autofocused a field. */
let prior: Element | null = null;
let current: Element | null = null;

if (typeof document !== 'undefined')
  document.addEventListener(
    'focusin',
    e => {
      if (e.target === current) return;
      prior = current;
      current = e.target as Element;
    },
    true
  );

/** The element to give focus back to once `layer` closes. */
function openerOf(layer: HTMLElement | null): HTMLElement | null {
  const active = document.activeElement;
  const candidate = active && layer?.contains(active) ? prior : active;
  return candidate instanceof HTMLElement && candidate !== document.body ? candidate : null;
}

/**
 * Keeps keyboard focus with a modal layer: while `ref` is the top modal,
 * Tab and Shift+Tab wrap inside it instead of leaving for the window behind,
 * and when it closes, focus goes back to what had it when it opened. An open
 * menu (portalled to the body) keeps its own keys. `open` is for a layer
 * that stays mounted while closed.
 */
export function useModalFocus(ref: RefObject<HTMLElement | null>, open = true) {
  useEffect(() => {
    if (!open) return;
    const opener = openerOf(ref.current);
    const onKey = (e: KeyboardEvent) => {
      const layer = ref.current;
      if (e.key !== 'Tab' || !layer || !isTopModal(layer)) return;
      if (document.querySelector('[role="menu"]')) return;
      const items = tabbables(layer);
      const first = items[0];
      const last = items[items.length - 1];
      const now = document.activeElement;
      const inside = now instanceof Node && layer.contains(now);
      if (!first || !last) {
        e.preventDefault();
      } else if (!inside || (e.shiftKey ? now === first : now === last)) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      const now = document.activeElement;
      const lost = !now || now === document.body || !now.isConnected;
      if (opener?.isConnected && lost) opener.focus();
    };
  }, [ref, open]);
}
