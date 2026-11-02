import { useEffect } from 'react';
import { useAppStore } from './use-app-store';

/**
 * While `active`, looks at the tools again each time the window comes back:
 * that is when someone has just installed or moved one. Every caller shares
 * the store's throttle.
 */
export function useToolRecheckOnFocus(active: boolean) {
  useEffect(() => {
    if (!active) return;
    const look = () => void useAppStore.getState().recheckToolsOnFocus();
    window.addEventListener('focus', look);
    return () => window.removeEventListener('focus', look);
  }, [active]);
}
