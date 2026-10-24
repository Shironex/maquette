/**
 * The Concept lab of the current project: its directions, the Codex job
 * queue and the composer draft, kept fresh by `conceptUpdate` events.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type Aspect,
  type AssetKind,
  type ConceptEvent,
  type LabView,
} from '@/ipc/client';
import { useAppStore } from './app-store';
import { applyConceptEvent, withDirection } from './lab';

export interface Draft {
  name: string;
  prompt: string;
  /** Relative to the project folder, or absolute. */
  refs: string[];
  aspect: Aspect;
  count: number;
}

/** "Make object refs...": the concept to take objects from and what to make. */
export interface ObjectsDraft {
  project: string;
  /** Relative to the project folder. */
  source: string;
  /** The same file, absolute, for the picture. */
  sourcePath: string;
  /** The direction or asset the concept belongs to. */
  sourceName: string;
  objects: string[];
  /** Extra instructions for the prompt, by object name. Not part of the asset name. */
  notes: Record<string, string>;
  aspect: Aspect;
  kind: 'prop' | 'character';
}

const EMPTY_DRAFT: Draft = { name: '', prompt: '', refs: [], aspect: '16:9', count: 4 };

interface LabStore {
  view: LabView | null;
  loading: boolean;
  /** The direction shown in the results. */
  selected: string | null;
  draft: Draft;
  /** The image open in the lightbox, as an absolute path. */
  lightbox: string | null;
  /** What is running: `generate`, `sheet`, `send`, `move`, `objects` or null. */
  busy: string | null;
  load: (project: string) => Promise<void>;
  select: (direction: string) => void;
  /** Loads an earlier direction's brief into the composer. */
  reuse: (direction: string) => void;
  setDraft: (patch: Partial<Draft>) => void;
  /** Clears the brief for a new direction. */
  newDraft: () => void;
  addRefs: (paths: string[]) => Promise<void>;
  removeRef: (path: string) => void;
  generate: () => Promise<void>;
  cancel: (jobId: string) => Promise<void>;
  cancelAll: () => Promise<void>;
  clearFinished: () => Promise<void>;
  mark: (file: string, marks: { starred?: boolean; picked?: boolean }) => Promise<void>;
  contactSheet: () => Promise<void>;
  /** Copies the picks into refs/; resolves to the asset's id, or null when it failed. */
  /** Copies the picks into refs/. `quiet` skips the toast, for when the Send to 3D panel opens next. */
  sendPicks: (kind: AssetKind, quiet?: boolean) => Promise<string | null>;
  /** Takes the Pick mark off every picked concept of `direction`. */
  clearPicks: (direction: string) => Promise<void>;
  /** Moves an earlier direction, with its refs and Board card, into another project. */
  moveDirection: (direction: string, to: string) => Promise<boolean>;
  openLightbox: (path: string | null) => void;
  onEvent: (event: ConceptEvent) => void;
  objects: ObjectsDraft | null;
  openObjects: (
    draft: Pick<ObjectsDraft, 'project' | 'source' | 'sourcePath' | 'sourceName'>
  ) => void;
  setObjects: (patch: Partial<Pick<ObjectsDraft, 'objects' | 'notes' | 'aspect' | 'kind'>>) => void;
  closeObjects: () => void;
  /** Queues one Codex job per object; resolves true when they started. */
  makeObjects: () => Promise<boolean>;
  /**
   * Sends the picks of an objects direction to Refs, one asset per pick.
   * Resolves to the new assets' ids, or null when it failed.
   */
  sendObjectPicks: (quiet?: boolean) => Promise<string[] | null>;
}

const notify = (status: 'done' | 'failed' | 'info', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

/** The project the latest load asked for; an older answer is dropped. */
let requested: string | null = null;
/** False once a switch moved the lab on, so a late answer stays out. */
const stillOn = (project: string) => requested === project;

export const useLabStore = create<LabStore>((set, get) => ({
  view: null,
  loading: false,
  selected: null,
  draft: EMPTY_DRAFT,
  lightbox: null,
  busy: null,
  objects: null,

  load: async project => {
    requested = project;
    // A switch starts the lab clean: nothing selected, an empty composer.
    if (get().view?.project !== project && get().view) {
      set({
        view: null,
        selected: null,
        draft: { ...EMPTY_DRAFT, count: get().draft.count },
        lightbox: null,
        busy: null,
      });
    }
    // Make object refs reads a concept of the project it opened in.
    if (get().objects && get().objects?.project !== project) set({ objects: null });
    set({ loading: true });
    try {
      const view = await commands.labGet(project);
      if (requested !== project) return;
      const selected = get().selected;
      const keep = view.directions.some(d => d.id === selected);
      const first = keep ? selected : (view.directions[0]?.id ?? null);
      set({ view, loading: false, selected: first });
      // An untouched composer starts from the direction on show.
      const { draft } = get();
      if (first && !draft.name && !draft.prompt && draft.refs.length === 0) get().reuse(first);
    } catch (error) {
      if (requested !== project) return;
      set({ loading: false });
      notify('failed', 'Could not read the concepts', errorMessage(error));
    }
  },

  select: direction => set({ selected: direction }),

  reuse: id => {
    const direction = get().view?.directions.find(d => d.id === id);
    if (!direction) return;
    set({
      selected: id,
      draft: {
        name: direction.name,
        prompt: direction.prompt,
        refs: direction.refs,
        aspect: direction.aspect,
        count: get().draft.count,
      },
    });
  },

  setDraft: patch => set({ draft: { ...get().draft, ...patch } }),

  newDraft: () => set({ draft: { ...EMPTY_DRAFT, count: get().draft.count } }),

  addRefs: async paths => {
    const view = get().view;
    if (!view || paths.length === 0) return;
    try {
      // Files from outside the project are copied into refs/ first.
      const outside = paths.filter(p => !p.startsWith('refs/'));
      const copied = outside.length ? await commands.labImportRefs(view.project, outside) : [];
      const inside = paths.filter(p => p.startsWith('refs/'));
      const refs = [...new Set([...get().draft.refs, ...inside, ...copied])];
      set({ draft: { ...get().draft, refs } });
      if (copied.length && stillOn(view.project)) await get().load(view.project);
    } catch (error) {
      notify('failed', 'Could not add the reference', errorMessage(error));
    }
  },

  removeRef: path =>
    set({ draft: { ...get().draft, refs: get().draft.refs.filter(r => r !== path) } }),

  generate: async () => {
    const view = get().view;
    const { draft } = get();
    if (!view) return;
    set({ busy: 'generate' });
    try {
      const next = await commands.labGenerate({
        project: view.project,
        direction: draft.name,
        prompt: draft.prompt,
        refs: draft.refs,
        aspect: draft.aspect,
        count: draft.count,
      });
      const fresh = next.jobs.filter(j => !view.jobs.some(old => old.id === j.id));
      if (stillOn(view.project))
        set({ view: next, selected: fresh[0]?.direction ?? get().selected });
    } catch (error) {
      notify('failed', 'The concepts did not start', errorMessage(error));
    } finally {
      set({ busy: null });
    }
  },

  cancel: async jobId => {
    try {
      await commands.labCancel(jobId);
    } catch (error) {
      notify('failed', 'Could not cancel the job', errorMessage(error));
    }
  },

  cancelAll: async () => {
    const view = get().view;
    if (!view) return;
    try {
      await commands.labCancelAll(view.project);
    } catch (error) {
      notify('failed', 'Could not cancel the jobs', errorMessage(error));
    }
  },

  clearFinished: async () => {
    const view = get().view;
    if (!view) return;
    try {
      const next = await commands.labClearFinished(view.project);
      if (stillOn(view.project)) set({ view: next });
    } catch (error) {
      notify('failed', 'Could not clear the queue', errorMessage(error));
    }
  },

  mark: async (file, { starred = null, picked = null } = {}) => {
    const view = get().view;
    const selected = get().selected;
    if (!view || !selected) return;
    try {
      const direction = await commands.labMark(view.project, selected, file, starred, picked);
      const latest = get().view;
      if (latest?.project === view.project) set({ view: withDirection(latest, direction) });
    } catch (error) {
      notify('failed', 'Could not mark the concept', errorMessage(error));
    }
  },

  contactSheet: async () => {
    const view = get().view;
    const selected = get().selected;
    if (!view || !selected) return;
    set({ busy: 'sheet' });
    try {
      const path = await commands.labContactSheet(view.project, selected);
      notify('done', 'Contact sheet saved', 'contact_sheet.png sits next to the concepts.');
      set({ lightbox: path });
    } catch (error) {
      notify('failed', 'No contact sheet', errorMessage(error));
    } finally {
      set({ busy: null });
    }
  },

  clearPicks: async id => {
    // By id, not the selection: another direction may be on show by now.
    const view = get().view;
    const direction = view?.directions.find(d => d.id === id);
    if (!view || !direction) return;
    try {
      // One at a time: each mark rewrites the direction's file.
      for (const image of direction.images.filter(i => i.picked)) {
        const next = await commands.labMark(view.project, direction.id, image.file, null, false);
        const latest = get().view;
        if (latest?.project !== view.project) return;
        set({ view: withDirection(latest, next) });
      }
    } catch (error) {
      notify('failed', 'Could not clear the picks', errorMessage(error));
    }
  },

  sendPicks: async (kind, quiet = false) => {
    const view = get().view;
    const selected = get().selected;
    if (!view || !selected) return null;
    set({ busy: 'send' });
    try {
      const sent = await commands.labSendPicks(view.project, selected, kind);
      useAppStore.getState().setProject(sent.project);
      const latest = get().view;
      if (latest?.project === view.project) set({ view: withDirection(latest, sent.direction) });
      if (stillOn(view.project)) await get().load(view.project);
      await get().clearPicks(selected);
      const n = sent.copied.length;
      if (!quiet)
        notify(
          'done',
          `${n} ${n === 1 ? 'pick' : 'picks'} sent to Refs`,
          `${sent.asset.name} is at Refs on the Board.`
        );
      return sent.asset.id;
    } catch (error) {
      notify('failed', 'Could not send the picks', errorMessage(error));
      return null;
    } finally {
      set({ busy: null });
    }
  },

  moveDirection: async (direction, to) => {
    const view = get().view;
    if (!view || get().busy) return false;
    set({ busy: 'move' });
    try {
      const moved = await commands.labMoveDirection(view.project, direction, to);
      const app = useAppStore.getState();
      app.setProject(moved.from);
      app.setProject(moved.to);
      if (get().selected === direction) {
        set({ selected: null, draft: { ...EMPTY_DRAFT, count: get().draft.count } });
      }
      if (stillOn(view.project)) await get().load(view.project);
      const n = moved.direction.images.length;
      notify(
        'done',
        `${moved.direction.name} moved to ${moved.to.name}`,
        [
          `Its ${n} ${n === 1 ? 'concept' : 'concepts'}, picks and stars went along.`,
          moved.asset ? `${moved.asset.name} is on its Board.` : null,
        ]
          .filter(Boolean)
          .join(' ')
      );
      return true;
    } catch (error) {
      notify('failed', 'Could not move the direction', errorMessage(error));
      return false;
    } finally {
      set({ busy: null });
    }
  },

  openLightbox: path => set({ lightbox: path }),

  onEvent: event => {
    const view = get().view;
    if (!view) return;
    set({ view: applyConceptEvent(view, event) });
    if (
      event.type === 'job' &&
      event.job.state === 'failed' &&
      event.job.project === view.project
    ) {
      notify('failed', `Concept ${event.job.index} failed`, event.job.message ?? undefined);
    }
  },

  openObjects: draft =>
    set({ objects: { ...draft, objects: [], notes: {}, aspect: '1:1', kind: 'prop' } }),

  setObjects: patch => {
    const objects = get().objects;
    if (objects) set({ objects: { ...objects, ...patch } });
  },

  closeObjects: () => set({ objects: null }),

  makeObjects: async () => {
    const objects = get().objects;
    if (!objects || get().busy) return false;
    set({ busy: 'objects' });
    // Job events can land before the answer, so the old jobs are read first.
    const before = get().view?.jobs ?? [];
    try {
      const next = await commands.labMakeObjects({
        project: objects.project,
        source: objects.source,
        sourceName: objects.sourceName,
        objects: objects.objects,
        notes: objects.notes,
        aspect: objects.aspect,
        kind: objects.kind,
      });
      const fresh = next.jobs.filter(j => !before.some(old => old.id === j.id));
      // The Lab shows the new direction, also when this came from the Board,
      // unless another project was opened meanwhile.
      if (useAppStore.getState().workspace?.project?.id === objects.project) {
        requested = objects.project;
        set({ view: next, selected: fresh[0]?.direction ?? get().selected });
      }
      set({ objects: null });
      const n = objects.objects.length;
      notify(
        'info',
        `${n} object ${n === 1 ? 'ref is' : 'refs are'} on the way`,
        `Codex makes each one from ${objects.sourceName}. They land in ${fresh[0]?.directionName ?? 'the Lab'}.`
      );
      return true;
    } catch (error) {
      notify('failed', 'The object refs did not start', errorMessage(error));
      return false;
    } finally {
      set({ busy: null });
    }
  },

  sendObjectPicks: async (quiet = false) => {
    const view = get().view;
    const selected = get().selected;
    const direction = view?.directions.find(d => d.id === selected);
    if (!view || !direction) return null;
    set({ busy: 'send' });
    try {
      const sent = await commands.labSendObjectPicks(view.project, direction.id, direction.kind);
      useAppStore.getState().setProject(sent.project);
      const latest = get().view;
      if (latest?.project === view.project) set({ view: withDirection(latest, sent.direction) });
      if (stillOn(view.project)) await get().load(view.project);
      await get().clearPicks(direction.id);
      const n = sent.assets.length;
      if (!quiet)
        notify(
          'done',
          `${n} ${n === 1 ? 'object' : 'objects'} sent to Refs`,
          `${n === 1 ? 'It is' : 'Each is its own asset'} at Refs on the Board, named after the object.`
        );
      return sent.assets.map(a => a.asset.id);
    } catch (error) {
      notify('failed', 'Could not send the picks', errorMessage(error));
      return null;
    } finally {
      set({ busy: null });
    }
  },
}));
