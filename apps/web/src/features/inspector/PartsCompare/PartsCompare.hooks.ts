import { useEffect, useState } from 'react';
import {
  commands,
  errorMessage,
  type Asset,
  type AssetParts,
  type ModelParts,
  type PartsCompare,
  type PartsSide,
} from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { compareRows, sidesFor } from '@/state/parts-compare';
import { useAppStore } from '@/state/use-app-store';

export type ComparePartsMode = 'shaded' | 'parts' | 'wireframe';

/**
 * Before and after a split, side by side: the newest model as it came and
 * after the split in Blender, or the version before against the newest.
 * Keep makes one side the one the Board and the next export use.
 */
export function usePartsCompare(asset: Asset, found: AssetParts, onChanged: () => void) {
  const setProject = useAppStore(s => s.setProject);
  const sides = sidesFor(asset, found);
  const key = sides ? JSON.stringify(sides) : null;
  const [report, setReport] = useState<PartsCompare | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [split, setSplit] = useState(0.5);
  const [mode, setMode] = useState<ComparePartsMode>('parts');
  const [spread, setSpread] = useState(0);

  useEffect(() => {
    setReport(null);
    if (!key) return;
    const [left, right] = JSON.parse(key) as [PartsSide, PartsSide];
    let live = true;
    setError(null);
    commands
      .partsCompare(found.project, found.asset, left, right)
      .then(r => live && setReport(r))
      .catch(e => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [found.project, found.asset, key]);

  /** Keep a side: a version moves to the end; keeping as it came drops the split. */
  const keep = async (side: PartsSide) => {
    setBusy(true);
    setError(null);
    try {
      if (side.fixed) {
        useAppStore.getState().notify({
          status: 'done',
          title: `${asset.name} keeps its split`,
          detail: 'The next build ships its parts by the names they have here.',
        });
      } else if (found.split && sides) {
        const { entry } = await commands.prepareAsset(found.project, found.asset);
        const fixups = entry.fixups.filter(f => f.op !== 'split-parts' && f.op !== 'merge');
        await commands.prepareSave(found.project, found.asset, { ...entry, fixups });
        useAppStore.getState().notify({
          status: 'done',
          title: `${asset.name} stays one mesh`,
          detail: 'The split is off; the next build ships the model as it came.',
        });
        onChanged();
      } else {
        setProject(await commands.partsKeepVersion(found.project, found.asset, side.version));
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const numbers: [ModelParts, ModelParts] | null = report ? [report.left, report.right] : null;
  return {
    sides,
    report,
    rows: numbers ? compareRows(numbers[0], numbers[1]) : [],
    urls: [fileUrl(report?.leftFile), fileUrl(report?.rightFile)] as const,
    error,
    busy,
    split,
    setSplit,
    mode,
    setMode,
    spread,
    setSpread,
    keep,
  };
}
