import { useEffect, useRef } from 'react';
import type { ScannedModel } from '@/ipc/client';
import { formatBytes, formatK } from '@/lib/format';
import { useModalFocus } from '@/lib/useModalFocus';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';

/** What the Check column says about a scanned model. */
export function checkText(model: ScannedModel): { tone: 'ok' | 'over' | 'note'; text: string } {
  if (model.error) return { tone: 'over', text: model.error };
  if (model.asset) return { tone: 'note', text: 'Imported' };
  const problems = model.budget?.problems ?? [];
  if (problems.length) return { tone: 'over', text: problems.join(', ') };
  const stats = model.stats;
  if (stats?.joints) {
    const clips = stats.animations;
    return { tone: 'note', text: `Rigged, ${clips} ${clips === 1 ? 'clip' : 'clips'}` };
  }
  return { tone: 'ok', text: model.budget ? 'Fits the budget' : 'No budget for this folder' };
}

/** The scan, what is picked and the totals the header shows. */
export function useGameImportDialog() {
  const store = useGameStore();
  const project = useAppStore(s => s.workspace?.project ?? null);
  const repo = useAppStore(s => s.workspace?.project?.siteRepo ?? null);
  const home = useAppStore(s => s.boot?.home);
  const sheet = useRef<HTMLFormElement>(null);
  const open = Boolean(store.importFor);
  const { closeImport } = store;
  useModalFocus(sheet, open);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeImport();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, closeImport]);

  const models = store.scan?.models ?? [];
  const picked = new Set(store.picked);
  const over = models.filter(m => !m.asset && (m.budget?.problems.length ?? 0) > 0).length;
  const bytes = models.reduce((sum, m) => sum + (m.stats?.bytes ?? 0), 0);
  const seconds = store.scan?.millis ? (store.scan.millis / 1000).toFixed(1) : null;
  return {
    open,
    sheet,
    name: project?.name ?? 'the game',
    repo,
    home,
    scan: store.scan,
    scanning: store.scanning,
    models,
    picked,
    importing: store.importing,
    toggle: store.toggle,
    pickAll: store.pickAll,
    close: closeImport,
    submit: () => void store.runImport(),
    counts: {
      found: `${models.length} ${models.length === 1 ? 'model' : 'models'}`,
      selected: String(store.picked.length),
      over: String(over),
      disk: formatBytes(bytes),
    },
    scanned: store.scan
      ? `Scanned ${store.scan.modelsDir}/${seconds ? ` in ${seconds} s` : ''}.`
      : 'Scanning the model folder.',
    triangles: (model: ScannedModel) => (model.stats ? formatK(model.stats.triangles) : '-'),
    textures: (model: ScannedModel) =>
      model.stats?.textures ? `${model.stats.textures} · ${model.stats.maxTexture}` : 'none',
    size: (model: ScannedModel) => (model.stats?.bytes ? formatBytes(model.stats.bytes) : '-'),
  };
}
