import { commands, errorMessage } from '@/ipc/client';
import { copyText } from '@/features/config/copyText';
import { formatAgo, formatBytes } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { MATERIALS } from '@/state/avatar';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';

/** The material mode, where the file goes, and what the last build wrote. */
export function useExportPanel() {
  const view = useAvatarStore(s => s.view);
  const material = useAvatarStore(s => s.draft?.material ?? 'mtoon-soft');
  const setMaterial = useAvatarStore(s => s.setMaterial);
  const notify = useAppStore(s => s.notify);
  const now = useNow(60_000);
  const written = view?.export ?? null;
  return {
    view,
    material,
    setMaterial,
    hint: MATERIALS.find(m => m.value === material)?.hint ?? '',
    written,
    facts: written
      ? [
          { label: 'Size', value: written.bytes === null ? 'n/a' : formatBytes(written.bytes) },
          { label: 'Written', value: formatAgo(Date.parse(written.writtenAt), now) },
          { label: 'From', value: `v${written.sourceVersion}`, mono: true },
          {
            label: 'Rest pose',
            value: written.normalized ? 'Normalised' : 'As modelled',
          },
          {
            label: 'Material',
            value: MATERIALS.find(m => m.value === written.material)?.label ?? written.material,
          },
        ]
      : [],
    reveal: () => {
      if (!written) return;
      commands.workspaceReveal(written.path).catch((error: unknown) =>
        notify({
          status: 'failed',
          title: 'Could not show the file',
          detail: errorMessage(error),
        })
      );
    },
    copy: () => {
      if (!written) return;
      void copyText(written.path).then(ok =>
        notify(
          ok
            ? { status: 'done', title: 'Path copied' }
            : {
                status: 'failed',
                title: 'Could not copy',
                detail: 'The clipboard is not available here.',
              }
        )
      );
    },
  };
}
