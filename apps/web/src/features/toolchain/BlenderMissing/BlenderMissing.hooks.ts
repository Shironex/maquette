import { useEffect } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { PLATFORM } from '@/ipc/runtime';
import { copyText } from '@/features/config/copyText';
import { useAppStore } from '@/state/use-app-store';
import { BLENDER_DOWNLOAD, blenderInstallCommand } from '@/state/toolchain';

/**
 * What the Blender screen shows, and a new look for Blender each time the
 * window comes back, because that is when someone has just installed it.
 */
export function useBlenderMissing() {
  const report = useAppStore(s => s.tools.reports.find(r => r.id === 'blender'));
  const checking = useAppStore(s => s.tools.checking);
  const notify = useAppStore(s => s.notify);
  const showing = report !== undefined && report.status !== 'found';
  useEffect(() => {
    if (!showing) return;
    const look = () => {
      const { tools, detectTools } = useAppStore.getState();
      if (!tools.checking) void detectTools();
    };
    window.addEventListener('focus', look);
    return () => window.removeEventListener('focus', look);
  }, [showing]);
  const command = blenderInstallCommand(PLATFORM);
  return {
    showing,
    broken: report?.status === 'broken',
    checking,
    command,
    getBlender: () => {
      commands.appOpenLink(BLENDER_DOWNLOAD).catch((error: unknown) =>
        notify({
          status: 'failed',
          title: 'Could not open blender.org',
          detail: errorMessage(error),
        })
      );
    },
    copyCommand: async () => {
      if (!command) return;
      const copied = await copyText(command);
      notify(
        copied
          ? { status: 'done', title: 'Command copied', detail: 'Run it in a terminal.' }
          : {
              status: 'failed',
              title: 'Could not copy',
              detail: 'The clipboard is not available here.',
            }
      );
    },
  };
}
