import { useEffect, useRef } from 'react';
import type { ImageBackend, ImageSettings, WorkspaceConfig } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { imageSettings, type ImageJobKind } from '@/state/image-api';
import { useImageApiStore } from '@/state/use-image-api-store';
import { useKeyDraft } from '@/features/providers/KeyFields';

/** The form with `images` in place of its image settings. */
function withImages(config: WorkspaceConfig, images: ImageSettings): WorkspaceConfig {
  return { ...config, lab: { ...config.lab, images } };
}

/**
 * The image settings in the form, the OpenAI key's row and the month's
 * spend. A change an agent made (`set_image_backend`) comes into the form.
 */
export function useImagesSection(
  config: WorkspaceConfig,
  onChange: (config: WorkspaceConfig) => void
) {
  const status = useImageApiStore(s => s.status);
  const busy = useImageApiStore(s => s.busy);
  const load = useImageApiStore(s => s.load);
  const saveKey = useImageApiStore(s => s.saveKey);
  const removeKey = useImageApiStore(s => s.removeKey);
  const testKey = useImageApiStore(s => s.testKey);
  const settings = imageSettings(config.lab.images);

  const set = (patch: Partial<ImageSettings>) =>
    onChange(withImages(config, { ...settings, ...patch }));
  const setBackend = (kind: ImageJobKind, backend: ImageBackend) =>
    set({ backends: { ...settings.backends, [kind]: backend } });

  useEffect(() => {
    void load();
  }, [load, config.lab.images]);

  const latest = useRef({ config, onChange });
  useEffect(() => {
    latest.current = { config, onChange };
  });
  useEffect(() => {
    // Without the event bridge (a test, the browser) the form just does not follow agents.
    const stop = events.imageSettingsChanged
      .listen(e => {
        const { config: now, onChange: change } = latest.current;
        change(withImages(now, e.payload));
      })
      .catch(() => null);
    return () => void stop.then(fn => fn?.());
  }, []);

  const key = useKeyDraft({
    status: status?.key,
    busy,
    save: saveKey,
    remove: removeKey,
    test: testKey,
  });

  return { settings, status, set, setBackend, key };
}
