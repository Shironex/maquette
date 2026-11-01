import { useState } from 'react';
import { clock } from '@/lib/clock';
import { useWait } from '@/lib/waiting';
import { formatAgo } from '@/lib/format';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';

/** The settings on show, the preset picker and its Save as preset, and where the pack goes. */
export function usePackSettings() {
  const s = usePackStore();
  const estimate = useWait('pack-plan', !s.settings && !s.error, Boolean(s.error));
  const root = useAppStore(st => st.workspace?.root) ?? '';
  const [naming, setNaming] = useState(false);
  const [presetName, setPresetName] = useState('');
  const current = s.presets.find(p => p.id === s.preset);
  const mixamo = s.plan?.mixamo ?? 0;
  const presetNote = current
    ? current.builtIn
      ? ['Comes with Maquette', current.used ? `used ${current.used}x` : null]
          .filter(Boolean)
          .join(', ')
      : [
          'Your own',
          current.savedAt ? `saved ${formatAgo(Date.parse(current.savedAt), clock.nowMs())}` : null,
          current.used ? `used ${current.used}x` : null,
        ]
          .filter(Boolean)
          .join(', ')
    : undefined;

  return {
    estimate,
    ...s,
    naming,
    presetName,
    setPresetName,
    presetNote,
    presetOptions: s.presets.map(p => ({ value: p.id, label: p.name })),
    where: s.out ?? `${root}/exports`,
    mixamoNote: mixamo
      ? `${mixamo === 1 ? '1 model is' : `${mixamo} models are`} on a Mixamo rig: Mixamo clips are never shared, so they stay out and the bones stay in.`
      : "Clips that may be Mixamo's always stay out; the bones stay in.",
    startNaming: () => setNaming(true),
    stopNaming: () => {
      setNaming(false);
      setPresetName('');
    },
    savePreset: async () => {
      if (!presetName.trim()) return;
      await s.savePreset(presetName);
      setNaming(false);
      setPresetName('');
    },
  };
}
