import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  commands,
  errorMessage,
  type AssetParts,
  type Fixup,
  type ObjectStates,
  type PartsPlan,
} from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { isGame } from '@/state/game';
import { MERGE_UNDER } from '@/state/prepare';
import {
  mergeParts,
  planOf,
  planProblem,
  renamePart,
  toggleHidden,
  unmergePart,
} from '@/state/parts';
import { useAppStore } from '@/state/use-app-store';

export type PartsMode = 'parts' | 'shaded' | 'wireframe';
/** The parts themselves, or before and after the split side by side. */
export type PartsPage = 'parts' | 'compare';

/** One place a part name is used: an object state or an explode track. */
export interface PartUse {
  label: string;
  parts: string;
}

/** The object states and explode track that pick `asset`'s parts by name. */
function usesOf(states: ObjectStates | undefined, explode: boolean, count: number): PartUse[] {
  const uses: PartUse[] = (states?.states ?? [])
    .filter(s => Object.keys(s.parts).length > 0)
    .map(s => ({ label: s.name, parts: Object.keys(s.parts).join(', ') }));
  if (explode) uses.push({ label: 'Explode', parts: `all ${count}` });
  return uses;
}

/**
 * The Parts tab: one model's parts as it ships them, selected in the viewer
 * or the list, renamed, joined and hidden. Every change saves at once in
 * `prepare.json`; the next build ships the names and joins.
 */
export function usePartsPanel() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const projectId = project?.id ?? null;
  const models = useMemo(
    () => (project?.assets ?? []).filter(a => a.kind !== 'scene' && a.versions.length > 0),
    [project?.assets]
  );
  const [picked, setPicked] = useState<string | null>(null);
  const asset = models.find(a => a.id === picked) ?? models[0] ?? null;
  const assetId = asset?.id ?? null;
  const [found, setFound] = useState<AssetParts | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [mode, setMode] = useState<PartsMode>('parts');
  const [page, setPage] = useState<PartsPage>('parts');
  const [uses, setUses] = useState<PartUse[]>([]);

  const load = useCallback(async (projectId: string, assetId: string, run: boolean) => {
    setBusy(true);
    setError(null);
    try {
      const got = run
        ? await commands.partsPreview(projectId, assetId)
        : await commands.partsGet(projectId, assetId);
      setFound(got);
      if (got.needsPreview && !run) {
        setFound(await commands.partsPreview(projectId, assetId));
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    setSelected([]);
    setFound(null);
    if (projectId && assetId) void load(projectId, assetId, false);
  }, [projectId, assetId, load]);

  const rowCount = found?.rows.length ?? 0;
  const game = isGame(project);
  useEffect(() => {
    if (!projectId || !assetId || game) return setUses([]);
    let live = true;
    commands
      .layoutGet(projectId)
      .then(view => {
        const explode = Boolean(view.layout.timeline.explode[assetId]);
        if (live) setUses(usesOf(view.layout.states[assetId], explode, rowCount));
      })
      .catch(() => live && setUses([]));
    return () => {
      live = false;
    };
  }, [projectId, assetId, game, rowCount]);

  const plan = planOf(found?.plan);
  const save = async (next: PartsPlan) => {
    if (!projectId || !found) return;
    const problem = planProblem(found.rows, next);
    if (problem) return setError(problem);
    setError(null);
    try {
      setFound(await commands.partsSave(projectId, found.asset, next));
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  /** A click in the viewer or the list: Shift adds to the selection. */
  const pick = (id: string | null, adding: boolean) => {
    if (!id) return setSelected([]);
    setSelected(now =>
      adding ? (now.includes(id) ? now.filter(s => s !== id) : [...now, id]) : [id]
    );
  };

  const rows = found?.rows ?? [];
  const chosen = rows.filter(r => selected.includes(r.id));
  const hidden = rows.filter(r => r.hidden).map(r => r.id);
  // The viewer names parts the way the file has them: a join shows as its parts.
  const inFile = (ids: readonly string[]) =>
    ids.flatMap(id => [id, ...(rows.find(r => r.id === id)?.joined ?? [])]);

  const addSplit = async () => {
    if (!projectId || !assetId) return;
    setBusy(true);
    try {
      const { entry } = await commands.prepareAsset(projectId, assetId);
      const fixups: Fixup[] = entry.fixups.filter(f => f.op !== 'merge' && f.op !== 'split-parts');
      fixups.push({ op: 'split-parts', by: 'loose', mergeUnder: MERGE_UNDER });
      await commands.prepareSave(projectId, assetId, { ...entry, fixups });
    } catch (e) {
      setBusy(false);
      return setError(errorMessage(e));
    }
    await load(projectId, assetId, true);
  };

  const shipped = rows.map(r => r.name);
  return {
    models,
    asset,
    pickAsset: setPicked,
    found,
    rows,
    busy,
    error,
    uses,
    game,
    page,
    setPage,
    reload: () => {
      if (projectId && assetId) void load(projectId, assetId, false);
    },
    /** What the viewer shows and how. */
    view: {
      url: fileUrl(found?.file),
      mode,
      setMode,
      look: { selected: inFile(selected), hidden: inFile(hidden) },
      selectedNames: chosen.map(r => r.name),
      hiddenNames: rows.filter(r => r.hidden).map(r => r.name),
    },
    selected,
    chosen,
    pick,
    shipped,
    /** Changes to the plan; each saves at once. */
    edit: {
      rename: (id: string, name: string) => void save(renamePart(plan, id, name)),
      merge: () => {
        void save(mergeParts(plan, selected));
        setSelected(selected.slice(0, 1));
      },
      unmerge: (id: string) => void save(unmergePart(plan, id)),
      hide: () => void save(toggleHidden(plan, selected)),
      allHidden: selected.length > 0 && chosen.every(r => r.hidden),
      /** Adds the split fix-up by loose pieces and runs it on a copy. */
      addSplit,
    },
  };
}
