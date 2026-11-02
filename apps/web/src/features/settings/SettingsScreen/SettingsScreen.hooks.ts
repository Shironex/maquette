import { useCallback, useEffect, useState } from 'react';
import { useRenderStore } from '@/state/use-render-store';
import { useUpdaterStore } from '@/state/use-updater-store';
import { useSettingsGuard } from '@/state/settings-guard';
import { useSettingsTarget } from '@/state/use-settings-target';
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
  useRevealToolchain(go);
  useRevealTarget(go);
  return { ...autosave, leaveTo, active, go };
}

/** Jumps to Toolchain when a render's failure asked for it. */
function useRevealToolchain(go: (id: SectionId) => void) {
  const asked = useRenderStore(s => s.toolchainAsked);
  const done = useRenderStore(s => s.toolchainShown);
  useEffect(() => {
    if (!asked) return;
    done();
    go('toolchain');
  }, [asked, done, go]);
}

/** Jumps to the section a button elsewhere asked for. */
function useRevealTarget(go: (id: SectionId) => void) {
  const asked = useSettingsTarget(s => s.asked);
  const shown = useSettingsTarget(s => s.shown);
  useEffect(() => {
    if (!asked) return;
    shown();
    go(asked);
  }, [asked, shown, go]);
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
