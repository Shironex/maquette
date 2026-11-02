import { useEffect, useId, useState } from 'react';
import { commands, type Direction } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import { pickRows, type LinkPick, type PickRow } from '@/state/plan-links';
import { useAppStore } from '@/state/use-app-store';
import { useStageLabels } from '@/state/use-stage-labels';

/**
 * The Asset picker's rows: the Board's assets, live from the open project,
 * and the Lab's directions, read once when it opens; a search over both
 * and the rows ticked to add.
 */
export function useAssetPicker(
  linked: { assets: readonly string[]; directions: readonly string[] },
  onAdd: (picked: LinkPick[]) => void
) {
  const workspace = useAppStore(s => s.workspace);
  const project = workspace?.project ?? null;
  const projectId = project?.id ?? null;
  const stageLabels = useStageLabels();
  const [directions, setDirections] = useState<Direction[] | null>(null);
  const [query, setQuery] = useState('');
  const [ticked, setTicked] = useState<string[]>([]);
  const search = useId();
  useEffect(() => {
    document.getElementById(search)?.focus();
  }, [search]);
  useEffect(() => {
    if (!projectId) return;
    let live = true;
    commands
      .labGet(projectId)
      .then(view => live && setDirections(view.directions))
      .catch(() => live && setDirections([]));
    return () => {
      live = false;
    };
  }, [projectId]);
  const rows = pickRows(
    { assets: project?.assets ?? [], directions: directions ?? [], stageLabels },
    query,
    linked
  );
  const picked = rows.filter(r => ticked.includes(r.key));
  return {
    search,
    query,
    setQuery,
    readingLab: directions === null,
    assets: rows.filter(r => r.kind === 'asset'),
    directions: rows.filter(r => r.kind === 'direction'),
    src: (row: PickRow) =>
      row.picture && workspace ? fileUrl(projectFile(workspace, row.picture)) : null,
    isTicked: (row: PickRow) => ticked.includes(row.key),
    tick: (row: PickRow, on: boolean) =>
      setTicked(t => (on ? [...t, row.key] : t.filter(k => k !== row.key))),
    count: picked.length,
    add: () => {
      if (picked.length) onAdd(picked.map(({ kind, id }) => ({ kind, id })));
    },
  };
}
