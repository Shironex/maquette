/**
 * The built in MCP server as the app sees it: running or not, its token,
 * the activity list and the approvals (shipping, sends to 3D), kept fresh
 * by `mcpUpdate`.
 */
import { create } from 'zustand';
import { commands, errorMessage, type McpStatus } from '@/ipc/client';
import { useAppStore } from './app-store';

interface AgentsStore {
  status: McpStatus | null;
  /** The approval being answered right now. */
  deciding: string | null;
  load: () => Promise<void>;
  onUpdate: (status: McpStatus) => void;
  decide: (approvalId: string, approve: boolean) => Promise<void>;
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

  decide: async (approvalId, approve) => {
    const approval = get().status?.approvals.find(a => a.id === approvalId);
    set({ deciding: approvalId });
    try {
      set({ status: await commands.mcpDecide(approvalId, approve) });
      if (approval?.kind === 'send-to-3d') {
        const asset = approval.assetName ?? 'The asset';
        notify(
          approve ? 'done' : 'info',
          approve ? `${asset} is on its way to ${approval.service}` : `${asset} was not sent`,
          approve
            ? 'Follow it in the 3D queue. The agent sees it on its next get_provider_job.'
            : 'Nothing was spent. The agent is told it was denied.'
        );
      } else if (approval) {
        notify(
          approve ? 'done' : 'info',
          approve ? `Run ${approval.runNumber} is shipping` : `Run ${approval.runNumber} stopped`,
          approve
            ? 'The agent sees the result on its next get_run.'
            : 'Ship did not run. The agent is told it was denied.'
        );
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
