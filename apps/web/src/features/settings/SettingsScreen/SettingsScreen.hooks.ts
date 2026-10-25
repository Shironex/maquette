import { useState } from 'react';
import { useSettingsGuard } from '@/state/settings-guard';
import type { SectionId } from '../sections';
import { useAutosave } from './useAutosave';

/** The autosaved draft, the section in view and the view Settings is asked to leave for. */
export function useSettingsScreen() {
  const autosave = useAutosave();
  const leaveTo = useSettingsGuard(s => s.leaveTo);
  const [active, setActive] = useState<SectionId>('workspace');
  const go = (id: SectionId) => {
    setActive(id);
    document
      .getElementById(`settings-${id}`)
      ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  };
  return { ...autosave, leaveTo, active, go };
}
