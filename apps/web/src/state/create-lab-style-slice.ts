import type { StateCreator } from 'zustand';
import { commands, errorMessage, type StyleBoard } from '@/ipc/client';
import { useAppStore } from './use-app-store';
import { MAX_STYLE_IMAGES } from './lab-style';
import type { LabStore } from './lab-store.types';

/** The style board: the dialog's draft, Save, and Add to style board from a concept. */
export type LabStyleSlice = Pick<
  LabStore,
  'style' | 'openStyle' | 'setStyle' | 'closeStyle' | 'saveStyle' | 'addToStyle'
>;

const notify = (status: 'done' | 'failed', title: string, detail?: string) =>
  useAppStore.getState().notify({ status, title, detail });

export const createLabStyleSlice: StateCreator<LabStore, [], [], LabStyleSlice> = (set, get) => {
  /** Puts a saved board into the view, unless another project is on show by now. */
  const show = (project: string, style: StyleBoard) => {
    const view = get().view;
    if (view?.project === project) set({ view: { ...view, style } });
  };

  return {
    style: null,

    openStyle: () => {
      const board = get().view?.style;
      if (!board) return;
      set({
        style: {
          line: board.line,
          palette: [...board.palette],
          images: board.images.map(i => i.file),
        },
      });
    },

    setStyle: patch => {
      const style = get().style;
      if (style) set({ style: { ...style, ...patch } });
    },

    closeStyle: () => set({ style: null }),

    saveStyle: async () => {
      const view = get().view;
      const style = get().style;
      if (!view || !style || get().busy) return false;
      set({ busy: 'style' });
      try {
        const saved = await commands.labSetStyle(view.project, {
          line: style.line,
          palette: style.palette,
          images: style.images,
        });
        show(view.project, saved);
        set({ style: null });
        notify(
          'done',
          'Style board saved',
          saved.revision === view.style.revision
            ? 'Nothing changed.'
            : `Revision ${saved.revision}. New directions and object refs carry it.`
        );
        return true;
      } catch (error) {
        notify('failed', 'Could not save the style board', errorMessage(error));
        return false;
      } finally {
        set({ busy: null });
      }
    },

    addToStyle: async file => {
      const view = get().view;
      if (!view || get().busy) return;
      const images = view.style.images;
      if (images.some(i => i.source === file || i.file === file)) return;
      if (images.length >= MAX_STYLE_IMAGES) {
        notify(
          'failed',
          'The style board is full',
          `It holds ${MAX_STYLE_IMAGES} images. Take one off in the style board first.`
        );
        return;
      }
      set({ busy: 'style' });
      try {
        const saved = await commands.labSetStyle(view.project, {
          images: [...images.map(i => i.file), file],
        });
        show(view.project, saved);
        notify('done', 'Added to the style board', `Revision ${saved.revision}.`);
      } catch (error) {
        notify('failed', 'Could not add it to the style board', errorMessage(error));
      } finally {
        set({ busy: null });
      }
    },
  };
};
