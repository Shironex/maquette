/**
 * The question asked before opening a workspace another computer has open
 * right now (its lock is fresh): open it here anyway, or not. Opening a
 * shared workspace on two computers at once makes the sync tool keep
 * conflict copies of the files both change.
 */
import { create } from 'zustand';
import { commands, isErrorPayload, type WorkspaceSnapshot } from '@/ipc/client';

/** The error code `workspace_open` answers with while the workspace is in use. */
export const WORKSPACE_IN_USE = 'workspace-in-use';

export interface LockQuestion {
  root: string;
  /** "This workspace is open on Studio PC. Close Maquette there first, ..." */
  message: string;
}

interface WorkspaceLockStore {
  question: LockQuestion | null;
  /** Asks; resolves true for Open anyway, false for Cancel. */
  ask: (question: LockQuestion) => Promise<boolean>;
  answer: (open: boolean) => void;
}

let pending: ((open: boolean) => void) | null = null;

export const useWorkspaceLock = create<WorkspaceLockStore>(set => ({
  question: null,
  ask: question => {
    pending?.(false);
    set({ question });
    return new Promise<boolean>(resolve => {
      pending = resolve;
    });
  },
  answer: open => {
    const resolve = pending;
    pending = null;
    set({ question: null });
    resolve?.(open);
  },
}));

/**
 * Opens the workspace at `root`, asking first when another computer has it
 * open. `null` when the person chose Cancel.
 */
export async function openWorkspace(root: string): Promise<WorkspaceSnapshot | null> {
  try {
    return await commands.workspaceOpen(root, false);
  } catch (error) {
    if (!isErrorPayload(error) || error.code !== WORKSPACE_IN_USE) throw error;
    return openAfterAsking({ root, message: error.message });
  }
}

/** Asks `question`, then opens anyway or gives `null`. */
export async function openAfterAsking(question: LockQuestion): Promise<WorkspaceSnapshot | null> {
  const lock = useWorkspaceLock.getState();
  if (!(await lock.ask(question))) return null;
  return commands.workspaceOpen(question.root, true);
}
