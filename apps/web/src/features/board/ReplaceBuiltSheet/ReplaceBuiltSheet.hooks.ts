import { useEffect, useState } from 'react';
import { commands, errorMessage, type ReplaceReport, type UndoReport } from '@/ipc/client';
import type { ReplaceTarget } from '@/state/built-replace';
import { useAppStore } from '@/state/use-app-store';
import { useBuiltStore } from '@/state/use-built-store';

/** What the sheet read for the pair or asset it is open for. */
type Plan =
  | { kind: 'idle' }
  | { kind: 'reading' }
  | { kind: 'refused'; why: string }
  | { kind: 'replace'; report: ReplaceReport }
  | { kind: 'undo'; report: UndoReport };

/** The sheet's target, for the Board that mounts it while one is set. */
export function useReplaceTarget() {
  const target = useBuiltStore(s => s.replacing);
  const setReplacing = useBuiltStore(s => s.setReplacing);
  return { target, close: () => setReplacing(null) };
}

/**
 * The pair the sheet is open for, the plan read for it (nothing changes
 * until Replace or Bring it back) and the call. A pick reads the plan
 * again, and a plan read for an older pick is dropped.
 */
export function useReplaceBuiltSheet(project: string, target: ReplaceTarget) {
  const items = useBuiltStore(s => s.items);
  const replace = useBuiltStore(s => s.replace);
  const undoReplace = useBuiltStore(s => s.undoReplace);
  const setReplacing = useBuiltStore(s => s.setReplacing);
  const assets = useAppStore(s => s.workspace?.project?.assets);
  const [asset, setAsset] = useState(target.asset ?? '');
  const [built, setBuilt] = useState(target.mode === 'replace' ? (target.built ?? '') : '');
  const [plan, setPlan] = useState<Plan>({ kind: 'idle' });
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const undo = target.mode === 'undo';

  useEffect(() => {
    if (!asset || (!undo && !built)) return;
    let live = true;
    const read: Promise<Plan> = undo
      ? commands.builtReplaceUndoPlan(project, asset).then(report => ({ kind: 'undo', report }))
      : commands
          .builtReplacePlan(project, asset, built)
          .then(report => ({ kind: 'replace', report }));
    void read
      .catch((error: unknown): Plan => ({ kind: 'refused', why: errorMessage(error) }))
      .then(next => {
        if (live) setPlan(next);
      });
    return () => {
      live = false;
    };
  }, [project, asset, built, undo]);

  const pick = (set: (id: string) => void) => (id: string) => {
    set(id);
    setFailed(null);
    setPlan({ kind: 'reading' });
  };
  const builtIds = new Set(items.map(b => b.asset));
  const name = (id: string) => assets?.find(a => a.id === id)?.name ?? id;
  const confirm = async () => {
    setBusy(true);
    setFailed(null);
    try {
      if (undo) await undoReplace(project, asset);
      else await replace(project, asset, built);
    } catch (error) {
      setFailed(errorMessage(error));
      setBusy(false);
    }
  };
  const picked = Boolean(asset) && (undo || Boolean(built));
  // A read is on its way from the moment the pair is whole.
  const shown: Plan = picked && plan.kind === 'idle' ? { kind: 'reading' } : plan;

  return {
    undo,
    asset,
    built,
    assetName: name(asset),
    builtName: name(built),
    /** The built assets that can take a place. */
    builtOptions: items.map(b => ({ value: b.asset, label: b.name })),
    /** The assets with a model that no script builds. */
    assetOptions: (assets ?? [])
      .filter(a => a.versions.length > 0 && !builtIds.has(a.id))
      .map(a => ({ value: a.id, label: a.name })),
    setAsset: pick(setAsset),
    setBuilt: pick(setBuilt),
    plan: shown,
    busy,
    failed,
    confirm,
    close: () => setReplacing(null),
  };
}
