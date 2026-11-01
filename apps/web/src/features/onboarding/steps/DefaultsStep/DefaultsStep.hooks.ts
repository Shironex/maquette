import { useState } from 'react';
import type { WorkspaceConfig } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { modelRoutes } from '@/state/onboarding';

/**
 * Recommended until someone asks to edit. The chips read no keys: on a Mac
 * reading the keychain asks for a password, too much for a first look.
 */
export function useDefaultsStep(config: WorkspaceConfig) {
  const boot = useAppStore(s => s.boot);
  const tools = useAppStore(s => s.tools.reports);
  const [editing, setEditing] = useState(false);
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
