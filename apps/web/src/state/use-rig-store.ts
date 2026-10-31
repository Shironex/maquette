/**
 * The rig doctor: the case on show, the open cases, and every answer a
 * person gives it. The core runs the scripts on a copy; the case stays up to
 * date through the rig changed event, whether or not it is on show.
 */
import { create } from 'zustand';
import { commands, errorMessage, type RigRecord, type RigRun, type RigTarget } from '@/ipc/client';
import { useAppStore } from './use-app-store';

interface RigStore {
  /** The case on show, or null for the Inbox. */
  record: RigRecord | null;
  /** The cases with something left to do. */
  open: RigRecord[];
  busy: boolean;

  /** Opens the case of a model and shows it in the Inbox view. */
  show: (target: RigTarget) => Promise<void>;
  /** Shows a case by its id. */
  showCase: (id: string) => Promise<void>;
  /** Back to the Inbox. A run goes on. */
  leave: () => void;
  load: () => Promise<void>;
  accept: (project: string, asset: string | null) => Promise<void>;
  run: (what: RigRun) => Promise<void>;
  cancel: () => Promise<void>;
  decideJoints: (approve: boolean) => Promise<void>;
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

    show: async target => {
      try {
        const record = await commands.rigOpen(target);
        set({ record });
        get().onChanged(record);
        useAppStore.getState().setView('inbox');
      } catch (error) {
        failed('Could not open the rig doctor', error);
      }
    },
    showCase: async id => {
      try {
        set({ record: await commands.rigGet(id) });
        useAppStore.getState().setView('inbox');
      } catch (error) {
        failed('Could not open the rig doctor', error);
      }
    },
    leave: () => set({ record: null }),
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
      try {
        await commands.rigOpenBlender(record.id);
      } catch (error) {
        failed('Could not open Blender', error);
      }
    },
    onChanged: record =>
      set(s => ({
        record: s.record?.id === record.id ? record : s.record,
        open: isOpen(record)
          ? [record, ...s.open.filter(r => r.id !== record.id)]
          : s.open.filter(r => r.id !== record.id),
      })),
  };
});
