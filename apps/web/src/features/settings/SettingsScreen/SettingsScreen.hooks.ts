import { useCallback, useEffect, useState } from 'react';
import { useUpdaterStore } from '@/state/use-updater-store';
import { useSettingsGuard } from '@/state/settings-guard';
import type { SectionId } from '../sections';
import { useAutosave } from './useAutosave';

/** The autosaved draft, the section in view and the view Settings is asked to leave for. */
export function useSettingsScreen() {
  const autosave = useAutosave();
  const leaveTo = useSettingsGuard(s => s.leaveTo);
  const [active, setActive] = useState<SectionId>('workspace');
  const go = useCallback((id: SectionId) => {
    setActive(id);
    document
      .getElementById(`settings-${id}`)
      ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, []);
  useRevealAbout(go);
  return { ...autosave, leaveTo, active, go };
}

/** Jumps to About when the header's update pill asked for it. */
function useRevealAbout(go: (id: SectionId) => void) {
  const asked = useUpdaterStore(s => s.aboutAsked);
  const done = useUpdaterStore(s => s.aboutShown);
  useEffect(() => {
    if (!asked) return;
    done();
    go('about');
  }, [asked, done, go]);
}
