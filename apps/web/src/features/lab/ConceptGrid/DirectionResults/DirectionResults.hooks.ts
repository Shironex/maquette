import { useState } from 'react';
import type { Asset, AssetKind, Direction, LabView } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { inProject, isObjects, tiles } from '@/state/lab';
import { styleChip } from '@/state/lab-style';
import { useLabStore } from '@/state/use-lab-store';
import { useProvidersStore } from '@/state/use-providers-store';

const NO_ASSETS: readonly Asset[] = [];

/** The tiles, the picks, and Send picks to Refs and Send to 3D with the kind menu. */
export function useDirectionResults(view: LabView, direction: Direction) {
  const busy = useLabStore(s => s.busy);
  const contactSheet = useLabStore(s => s.contactSheet);
  const sendPicks = useLabStore(s => s.sendPicks);
  const sendObjectPicks = useLabStore(s => s.sendObjectPicks);
  const openObjects = useLabStore(s => s.openObjects);
  const assets = useAppStore(s => s.workspace?.project?.assets) ?? NO_ASSETS;
  const openPanel = useProvidersStore(s => s.openPanel);
  const openBatch = useProvidersStore(s => s.openBatch);
  const objects = isObjects(direction);
  const from = objects
    ? view.directions.find(d => d.images.some(i => i.file === direction.source))
    : undefined;
  /** Which button opened the kind menu: Refs only, or Refs then 3D. */
  const [kindMenu, setKindMenu] = useState<'refs' | '3d' | null>(null);
  const cells = tiles(direction, view.jobs);
  const picked = direction.images.filter(i => i.picked).length;
  const named = assets.find(
    a => a.name.trim().toLowerCase() === direction.name.trim().toLowerCase()
  );
  // A card made by dropping images only guessed its kind, so the send still asks.
  const known =
    named?.stage === 'concept' && named.thumbnail?.startsWith('concepts/') ? undefined : named;

  const send = () => {
    if (objects) void sendObjectPicks();
    else if (known) void sendPicks(known.kind);
    else setKindMenu(m => (m === 'refs' ? null : 'refs'));
  };
  // Picks already in refs/ go straight to the panel; new ones are sent first.
  const unsent = direction.images.some(i => i.picked && !i.refFile);
  const sendTo3d = async (kind?: AssetKind) => {
    const id = known && !unsent ? known.id : await sendPicks(kind ?? known?.kind ?? 'prop', true);
    if (id) void openPanel(view.project, id);
  };
  const to3d = () => {
    if (objects) {
      // One asset per object, then one batch for all of them.
      void sendObjectPicks(true).then(ids => {
        if (ids?.length) void openBatch(view.project, ids);
      });
    } else if (known) void sendTo3d();
    else setKindMenu(m => (m === '3d' ? null : '3d'));
  };
  /** Make object refs on the first pick: cutting spends Codex usage, so it only opens the sheet. */
  const cutFirstPick = () => {
    const first = direction.images.find(i => i.picked);
    if (!first) return;
    openObjects({
      project: view.project,
      source: first.file,
      sourcePath: inProject(view.projectDir, first.file),
      sourceName: direction.name,
    });
  };
  return {
    cutFirstPick,
    busy,
    contactSheet,
    sendPicks,
    objects,
    from,
    kindMenu,
    setKindMenu,
    cells,
    picked,
    known,
    send,
    unsent,
    sendTo3d,
    to3d,
    chip: styleChip(direction, view.style),
  };
}
