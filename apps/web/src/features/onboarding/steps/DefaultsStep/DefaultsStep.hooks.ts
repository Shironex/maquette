import { useState } from 'react';
import type { WorkspaceConfig } from '@/ipc/client';
import { deepEqual } from '@/lib/deepEqual';
import { useAppStore } from '@/state/use-app-store';
import { modelRoutes } from '@/state/onboarding';

const edited = (config: WorkspaceConfig, recommended: WorkspaceConfig) =>
  !deepEqual(config.providers, recommended.providers) ||
  !deepEqual(config.budgets, recommended.budgets) ||
  !deepEqual(config.compression, recommended.compression);

/**
 * Recommended until someone asks to edit, or comes back to values they edited. The chips read no keys: on a Mac
 * reading the keychain asks for a password, too much for a first look.
 */
export function useDefaultsStep(config: WorkspaceConfig) {
  const boot = useAppStore(s => s.boot);
  const tools = useAppStore(s => s.tools.reports);
  const [editing, setEditing] = useState(() => (boot ? edited(config, boot.defaultConfig) : false));
  return {
    editing,
    edit: () => setEditing(true),
    /** The presets, budgets and compression this computer started with. */
    recommended: (): WorkspaceConfig => {
      setEditing(false);
      if (!boot) return config;
      const { providers, budgets, compression } = boot.defaultConfig;
      return { ...config, providers, budgets, compression };
    },
    routes: modelRoutes(tools),
  };
}
