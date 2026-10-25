/**
 * Credits: the workspace's ledger of paid sends, each provider's balance
 * and the Credits sheet's filters. The ledger is read when the sheet opens;
 * the balances are asked for then too, and again on Refresh.
 */
import { create } from 'zustand';
import { commands, errorMessage, type Balance, type Ledger } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { csvName, filterSpends, spendsCsv, type CreditsFilter } from './credits';
import { useAppStore } from './use-app-store';

interface CreditsStore {
  open: boolean;
  ledger: Ledger | null;
  balances: Balance[] | null;
  /** Why the ledger could not be read. */
  error: string | null;
  loading: boolean;
  checking: boolean;
  filter: CreditsFilter;
  /** Opens the sheet, filtered to `project` when given. */
  openCredits: (project?: string | null) => void;
  close: () => void;
  load: () => Promise<void>;
  checkBalances: () => Promise<void>;
  setFilter: (patch: Partial<CreditsFilter>) => void;
  /** Saves the spends the filter shows as CSV. */
  exportCsv: () => Promise<void>;
}

const notify = (status: 'done' | 'failed', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

export const useCreditsStore = create<CreditsStore>((set, get) => ({
  open: false,
  ledger: null,
  balances: null,
  error: null,
  loading: false,
  checking: false,
  filter: { project: null, service: null, source: 'api' },

  openCredits: project => {
    set({ open: true, filter: { ...get().filter, project: project ?? null } });
    void get().load();
    void get().checkBalances();
  },

  close: () => set({ open: false }),

  load: async () => {
    set({ loading: true, error: null });
    try {
      set({ ledger: await commands.creditsLedger(), loading: false });
    } catch (error) {
      set({ loading: false, error: errorMessage(error) });
    }
  },

  checkBalances: async () => {
    if (get().checking) return;
    set({ checking: true });
    try {
      set({ balances: await commands.creditsBalances() });
    } catch (error) {
      notify('failed', 'Could not ask for the balances', errorMessage(error));
    } finally {
      set({ checking: false });
    }
  },

  setFilter: patch => set(s => ({ filter: { ...s.filter, ...patch } })),

  exportCsv: async () => {
    const { ledger, filter } = get();
    if (!ledger) return;
    const spends = filterSpends(ledger.spends, filter);
    try {
      const path = await commands.creditsSaveCsv(spendsCsv(spends), csvName(clock.nowMs()));
      if (path) notify('done', 'Credits saved as CSV', `${spends.length} sends in ${path}`);
    } catch (error) {
      notify('failed', 'Could not save the CSV', errorMessage(error));
    }
  },
}));
