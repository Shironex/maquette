import { vi } from 'vitest';
import type { Asset } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import type { BoardEnv } from './board.types';

/**
 * A Board environment for the card and column tests: the open mock project
 * and its jobs, with the selection and the card actions as spies.
 */
export function fakeBoardEnv(over: Partial<BoardEnv> = {}): BoardEnv {
  const workspace = useAppStore.getState().workspace;
  const project = workspace?.project;
  return {
    workspace,
    project,
    jobs: useProvidersStore.getState().jobs,
    otherProjects: (workspace?.projects ?? []).filter(p => p.id !== project?.id),
    dropJob: null,
    selection: {
      present: [],
      check: vi.fn(),
      shiftClick: vi.fn(() => vi.fn()),
      send: vi.fn(),
      clear: vi.fn(),
    },
    actions: {
      menuFor: null,
      setMenuFor: vi.fn(),
      copyFor: null,
      setCopyFor: vi.fn(),
      copying: false,
      renaming: null,
      setRenaming: vi.fn(),
      renamed: vi.fn(),
      sizing: null,
      setSizing: vi.fn(),
      later: vi.fn(),
      move: vi.fn(),
      dropOn: vi.fn(),
      copyTo: vi.fn(),
      openInBlender: vi.fn(),
    },
    ...over,
  };
}

/** An asset of the open mock project, by its name on the card. */
export function mockAsset(name: string): Asset {
  const asset = useAppStore.getState().workspace?.project?.assets?.find(a => a.name === name);
  if (!asset) throw new Error(`No mock asset called ${name}`);
  return asset;
}
