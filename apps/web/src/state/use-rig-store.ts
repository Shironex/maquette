/**
 * The rig doctor: the case on show, the open cases, and every answer a
 * person gives it. The core runs the scripts on a copy; the case stays up to
 * date through the rig changed event, whether or not it is on show.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type RigChainEdit,
  type RigRecord,
  type RigRoute,
  type RigRun,
  type RigTarget,
} from '@/ipc/client';
import { clock } from '@/lib/clock';
import { rememberRun } from '@/lib/waiting';
import { isRunning, runWait } from './rig';
import { openInBlender } from './open-in-blender';
import { useAppStore } from './use-app-store';

/** The name a rig doctor open is timed under. */
export const RIG_OPEN_WAIT = 'rig-open';

/** Where the rig doctor goes back to. */
export type RigFrom = 'board' | 'inbox';

interface RigStore {
  /** The case on show, or null for the Inbox. */
  record: RigRecord | null;
  /** The cases with something left to do. */
  open: RigRecord[];
  busy: boolean;
  /** A case being opened: shown at once, while its model's rig is read. */
  opening: { name: string } | null;
  /** The view the case on show came from. */
  from: RigFrom;

  /** Opens the case of a model called `name` and shows it in the Inbox view. */
  show: (target: RigTarget, name: string) => Promise<void>;
  /** Shows a case by its id. */
  showCase: (id: string) => Promise<void>;
  /** Back to the Inbox. A run goes on. */
  leave: () => void;
  load: () => Promise<void>;
  accept: (project: string, asset: string | null) => Promise<void>;
  run: (what: RigRun) => Promise<void>;
  cancel: () => Promise<void>;
  decideJoints: (approve: boolean) => Promise<void>;
  /** A person picks the other route; the case starts over. */
  setRoute: (route: RigRoute) => Promise<void>;
  /** A person's word on a creature's chains; a correction starts the case over. */
  confirmChains: (edits: RigChainEdit[]) => Promise<void>;
  land: () => Promise<void>;
  discard: () => Promise<void>;
  saveSheet: () => Promise<void>;
  openBlender: () => Promise<void>;
  onChanged: (record: RigRecord) => void;
}

const OVER = new Set<RigRecord['state']>(['landed', 'discarded', 'refused']);
const isOpen = (r: RigRecord) => !OVER.has(r.state);

const failed = (title: string, error: unknown) =>
  useAppStore.getState().notify({ status: 'failed', title, detail: errorMessage(error) });

/** Which open the store waits for; leaving, or a newer open, makes an answer late. */
let openTicket = 0;

/** The runs under way by case, timed so the next one can say how long this one took. */
const runs = new Map<string, { state: RigRecord['state']; at: number }>();

function timeRun(record: RigRecord) {
  const run = runs.get(record.id);
  if (isRunning(record)) {
    if (run?.state !== record.state)
      runs.set(record.id, { state: record.state, at: clock.nowMs() });
    return;
  }
  if (!run) return;
  runs.delete(record.id);
  const ended = record.state !== 'cancelled' && record.state !== 'stopped' && !record.error;
  if (ended) rememberRun(runWait(run.state), clock.nowMs() - run.at);
}

export const useRigStore = create<RigStore>((set, get) => {
  /** Runs one answer about the case on show and follows the record it gives. */
  const act = async (title: string, work: (id: string) => Promise<RigRecord>) => {
    const { record } = get();
    if (!record) return;
    set({ busy: true });
    try {
      get().onChanged(await work(record.id));
    } catch (error) {
      failed(title, error);
    } finally {
      set({ busy: false });
    }
  };
  return {
    record: null,
    open: [],
    busy: false,
    opening: null,
    from: 'inbox',

    show: async (target, name) => {
      const app = useAppStore.getState();
      const from: RigFrom = app.view === 'board' ? 'board' : 'inbox';
      const ticket = ++openTicket;
      const startedAt = clock.nowMs();
      set({ opening: { name }, from, record: null });
      app.setView('inbox');
      try {
        const record = await commands.rigOpen(target);
        rememberRun(RIG_OPEN_WAIT, clock.nowMs() - startedAt);
        // Left meanwhile: the case stays open, the Inbox's top bar leads back to it.
        const waited = ticket === openTicket && useAppStore.getState().view === 'inbox';
        if (ticket === openTicket) set({ opening: null });
        if (waited) set({ record });
        get().onChanged(record);
      } catch (error) {
        if (ticket !== openTicket) return;
        set({ opening: null });
        if (from === 'board') useAppStore.getState().setView('board');
        failed('Could not open the rig doctor', error);
      }
    },
    showCase: async id => {
      try {
        const record = await commands.rigGet(id);
        openTicket += 1;
        set({ record, opening: null, from: 'inbox' });
        useAppStore.getState().setView('inbox');
      } catch (error) {
        failed('Could not open the rig doctor', error);
      }
    },
    leave: () => {
      openTicket += 1;
      const { from } = get();
      set({ record: null, opening: null, from: 'inbox' });
      if (from === 'board') useAppStore.getState().setView('board');
    },
    load: async () => {
      try {
        set({ open: (await commands.rigList()).filter(isOpen) });
      } catch {
        set({ open: [] });
      }
    },
    accept: (project, asset) =>
      act('Could not accept the model', id => commands.rigAccept(id, project, asset)),
    run: what => act('Could not start the rig doctor', id => commands.rigRun(id, what)),
    cancel: async () => {
      const { record } = get();
      if (!record) return;
      try {
        await commands.rigCancel(record.id);
      } catch (error) {
        failed('Could not cancel the run', error);
      }
    },
    decideJoints: approve =>
      act('Could not answer the joint move', id => commands.rigDecideJoints(id, approve)),
    setRoute: route => act('Could not change the route', id => commands.rigSetRoute(id, route)),
    confirmChains: edits =>
      act('Could not save the chains', id => commands.rigConfirmChains(id, edits)),
    land: () => act('Could not add the draft', id => commands.rigLand(id)),
    discard: () => act('Could not discard the draft', id => commands.rigDiscard(id)),
    saveSheet: async () => {
      const { record } = get();
      if (!record) return;
      try {
        const file = await commands.rigSaveSheet(record.id);
        useAppStore.getState().notify({ status: 'info', title: 'Sheet saved', detail: file });
      } catch (error) {
        failed('Could not save the sheet', error);
      }
    },
    openBlender: async () => {
      const { record } = get();
      if (!record) return;
      await openInBlender(record.name, () => commands.rigOpenBlender(record.id));
    },
    onChanged: record => {
      timeRun(record);
      set(s => ({
        record: s.record?.id === record.id ? record : s.record,
        open: isOpen(record)
          ? [record, ...s.open.filter(r => r.id !== record.id)]
          : s.open.filter(r => r.id !== record.id),
      }));
    },
  };
});
