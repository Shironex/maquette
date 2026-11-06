/**
 * The built in MCP server as the app sees it: running or not, its token,
 * the activity list and the approvals (shipping, sends to 3D), kept fresh
 * by `mcpUpdate`.
 */
import { create } from 'zustand';
import { commands, errorMessage, type DecidedBy, type McpStatus } from '@/ipc/client';
import { approvalToast } from './approval-toast';
import { isGame } from './game';
import { useAppStore } from './use-app-store';

interface AgentsStore {
  status: McpStatus | null;
  /** The approval being answered right now. */
  deciding: string | null;
  load: () => Promise<void>;
  onUpdate: (status: McpStatus) => void;
  /** Answers an approval; `by` says which card and what pressed it, for the record. */
  decide: (approvalId: string, approve: boolean, by: DecidedBy) => Promise<void>;
  clearActivity: () => Promise<void>;
  renewToken: () => Promise<void>;
}

const notify = (status: 'done' | 'failed' | 'info', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

export const useAgentsStore = create<AgentsStore>((set, get) => ({
  status: null,
  deciding: null,

  load: async () => {
    try {
      set({ status: await commands.mcpStatus() });
    } catch (error) {
      notify('failed', 'Could not read the MCP server', errorMessage(error));
    }
  },

  onUpdate: status => set({ status }),

  decide: async (approvalId, approve, by) => {
    const approval = get().status?.approvals.find(a => a.id === approvalId);
    set({ deciding: approvalId });
    try {
      set({ status: await commands.mcpDecide(approvalId, approve, by) });
      if (approval) {
        const project = useAppStore
          .getState()
          .workspace?.projects.find(p => p.id === approval.project);
        const { status, title, detail } = approvalToast(approval, approve, isGame(project));
        notify(status, title, detail);
      }
    } catch (error) {
      notify('failed', 'Could not answer the approval', errorMessage(error));
      void get().load();
    } finally {
      set({ deciding: null });
    }
  },

  clearActivity: async () => {
    try {
      set({ status: await commands.mcpClearActivity() });
    } catch (error) {
      notify('failed', 'Could not clear the activity', errorMessage(error));
    }
  },

  renewToken: async () => {
    try {
      set({ status: await commands.mcpRenewToken() });
      notify('done', 'New token made', 'Copy the config again for each agent.');
    } catch (error) {
      notify('failed', 'Could not make a new token', errorMessage(error));
    }
  },
}));
