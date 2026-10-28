import { useMemo, useState } from 'react';
import { commands, errorMessage, type Asset, type Recipe } from '@/ipc/client';
import { notify } from '@/state/providers-store-helpers';
import {
  MAX_ASSEMBLE_PARTS,
  assembleProblem,
  movePart,
  nodeName,
  type DraftPart,
} from '@/state/split';
import { useAppStore } from '@/state/use-app-store';
import { useSplitStore, type AssembleTarget } from '@/state/use-split-store';

/** Where the parts go: where their models put them, or by numbers. */
export type Placement = 'model' | 'numbers';

const NEW_ASSET = '';

function draftOf(asset: Asset, given?: Partial<DraftPart>): DraftPart {
  return {
    asset: asset.id,
    version: null,
    name: nodeName(asset.name),
    pivot: 'bottom-centre',
    at: null,
    turn: 0,
    triangles: asset.versions.at(-1)?.stats?.triangles ?? 0,
    ...given,
  };
}

/** A draft from a kept recipe, for the parts the project still has. */
function fromRecipe(recipe: Recipe, models: readonly Asset[]): DraftPart[] {
  return recipe.parts.flatMap(p => {
    const asset = models.find(a => a.id === p.asset);
    return asset ? [draftOf(asset, { ...p, version: p.version ?? null })] : [];
  });
}

/**
 * Assemble one prop: the parts in node order with their pivots and places,
 * the prop's name and the asset it becomes a version of.
 */
export function useAssembleSheet() {
  const target = useSplitStore(s => s.assemble);
  const close = useSplitStore(s => s.closeAssemble);
  const project = useAppStore(s => s.workspace?.project ?? null);
  const setProject = useAppStore(s => s.setProject);
  const models = useMemo(
    () => (project?.assets ?? []).filter(a => a.kind !== 'scene' && a.versions.length > 0),
    [project?.assets]
  );
  const [name, setName] = useState('');
  const [into, setInto] = useState(NEW_ASSET);
  const [parts, setParts] = useState<DraftPart[]>([]);
  const [placement, setPlacement] = useState<Placement>('model');
  const [busy, setBusy] = useState(false);

  // The sheet starts afresh once per opening, from the parts it was opened with.
  const [opened, setOpened] = useState<AssembleTarget | null>(null);
  if (opened !== target) {
    setOpened(target);
    setParts(
      (target?.parts ?? []).flatMap(id => {
        const asset = models.find(a => a.id === id);
        return asset ? [draftOf(asset)] : [];
      })
    );
    setInto(NEW_ASSET);
    setPlacement('model');
    setName('');
  }

  /** Picking an existing prop picks up the recipe it was last assembled with. */
  const pickInto = async (id: string) => {
    setInto(id);
    if (!target || !id) return;
    const asset = models.find(a => a.id === id);
    if (asset) setName(nodeName(asset.name));
    try {
      const recipe = await commands.assembleRecipe(target.project, id);
      if (recipe) {
        setName(recipe.name);
        setParts(fromRecipe(recipe, models));
        if (recipe.parts.some(p => p.at)) setPlacement('numbers');
      }
    } catch (e) {
      notify('failed', 'Could not read the last assembly', errorMessage(e));
    }
  };

  const used = new Set(parts.map(p => p.asset));
  const addable = models.filter(a => !used.has(a.id) && a.id !== into);
  const add = (id: string) => {
    const asset = models.find(a => a.id === id);
    if (asset && parts.length < MAX_ASSEMBLE_PARTS) setParts(p => [...p, draftOf(asset)]);
  };
  const update = (at: number, patch: Partial<DraftPart>) =>
    setParts(p => p.map((x, i) => (i === at ? { ...x, ...patch } : x)));
  const remove = (at: number) => setParts(p => p.filter((_, i) => i !== at));
  const move = (at: number, by: -1 | 1) => setParts(p => movePart(p, at, by));

  const blocker = assembleProblem(name, parts);

  const assemble = async () => {
    if (!target || blocker) return;
    setBusy(true);
    try {
      const made = await commands.assembleProp({
        project: target.project,
        asset: into || null,
        name: name.trim(),
        parts: parts.map(({ triangles: _, ...p }) => ({
          ...p,
          name: p.name.trim(),
          at: placement === 'numbers' ? (p.at ?? [0, 0, 0]) : null,
          turn: placement === 'numbers' ? (p.turn ?? 0) : 0,
        })),
      });
      setProject(made.project);
      notify(
        'done',
        `${name.trim()} assembled as v${made.version.version}`,
        `${made.nodes.length} parts under one node, each named as given.`
      );
      close();
    } catch (e) {
      notify('failed', 'Could not assemble the prop', errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return {
    target,
    models,
    name,
    setName,
    into,
    pickInto: (id: string) => void pickInto(id),
    parts,
    addable,
    add,
    update,
    remove,
    move,
    placement,
    setPlacement,
    blocker,
    busy,
    assemble: () => void assemble(),
    close,
  };
}

export type AssembleSheetModel = ReturnType<typeof useAssembleSheet>;
