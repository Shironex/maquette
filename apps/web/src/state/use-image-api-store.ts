/**
 * OpenAI's Image API as Settings and the Lab see it: the saved key (its
 * last four characters only), the month's spend against the cap, and the
 * key's Save, Remove and Test (which only lists the models, so it costs
 * nothing). The backends, quality and cap themselves are part of the
 * workspace settings form.
 */
import { create } from 'zustand';
import { commands, errorMessage, type ImageApiStatus, type OpenAiKeyTest } from '@/ipc/client';
import { notify } from './providers-store-helpers';

interface ImageApiStore {
  status: ImageApiStatus | null;
  /** A key call is on its way. */
  busy: boolean;
  load: () => Promise<void>;
  saveKey: (key: string) => Promise<boolean>;
  removeKey: () => Promise<void>;
  testKey: (key: string | null) => Promise<OpenAiKeyTest>;
}

export const useImageApiStore = create<ImageApiStore>((set, get) => {
  const keyCall = async <T>(call: () => Promise<T>): Promise<T> => {
    set({ busy: true });
    try {
      return await call();
    } finally {
      set({ busy: false });
    }
  };

  return {
    status: null,
    busy: false,

    load: async () => {
      try {
        set({ status: await commands.imageApiStatus() });
      } catch {
        // Without a workspace there is nothing to show yet.
      }
    },

    saveKey: key =>
      keyCall(async () => {
        try {
          await commands.imageApiKeySave(key);
          notify('done', 'OpenAI key saved', 'It is in the keychain, not in any file.');
          await get().load();
          return true;
        } catch (error) {
          notify('failed', 'Could not save the OpenAI key', errorMessage(error));
          return false;
        }
      }),

    removeKey: () =>
      keyCall(async () => {
        try {
          await commands.imageApiKeyRemove();
          notify('info', 'OpenAI key removed', 'It is gone from the keychain.');
          await get().load();
        } catch (error) {
          notify('failed', 'Could not remove the OpenAI key', errorMessage(error));
        }
      }),

    testKey: key =>
      keyCall(async () => {
        try {
          return await commands.imageApiKeyTest(key);
        } catch (error) {
          return { ok: false, hasModel: false, message: errorMessage(error) };
        }
      }),
  };
});
