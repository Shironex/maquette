/** Which Settings section a button elsewhere asked to land on. */
import { create } from 'zustand';
import { useAppStore } from './use-app-store';

export type SettingsTarget = 'toolchain' | 'project';

export const useSettingsTarget = create<{
  asked: SettingsTarget | null;
  ask: (section: SettingsTarget) => void;
  shown: () => void;
}>(set => ({
  asked: null,
  ask: asked => set({ asked }),
  shown: () => set({ asked: null }),
}));

/** Opens Settings at a section. */
export function useOpenSettings() {
  const setView = useAppStore(s => s.setView);
  const ask = useSettingsTarget(s => s.ask);
  return (section: SettingsTarget) => {
    ask(section);
    setView('settings');
  };
}
