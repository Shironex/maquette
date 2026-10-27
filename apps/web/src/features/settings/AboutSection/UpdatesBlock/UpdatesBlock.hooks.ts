import { useUpdaterStore } from '@/state/use-updater-store';
import { downloadPercent, phaseLine, phaseUpdate } from '@/state/updater';

/** The updater's state and steps, as the About section shows them. */
export function useUpdatesBlock(version: string) {
  const mode = useUpdaterStore(s => s.mode);
  const phase = useUpdaterStore(s => s.phase);
  const refused = useUpdaterStore(s => s.refused);
  const check = useUpdaterStore(s => s.check);
  const download = useUpdaterStore(s => s.download);
  const install = useUpdaterStore(s => s.install);
  return {
    mode,
    phase,
    refused,
    line: phaseLine({ mode, phase }, version),
    update: phaseUpdate(phase),
    percent: downloadPercent(phase),
    check: () => void check(),
    download: () => void download(),
    install: () => void install(),
  };
}
