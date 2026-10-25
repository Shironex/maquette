import { useEffect, useState } from 'react';
import type { InspectModel, MaterialTweak } from '@/ipc/client';
import {
  ALL_MATERIALS,
  dropsMap,
  emptyLook,
  isEmptyLook,
  lookOf,
  parseHex,
  tweakOf,
  withTweak,
} from '@/state/materials';
import { useMaterialsStore } from '@/state/use-materials-store';

/**
 * The Material panel's state: the asset's look as edited, the material the
 * fields edit (every one, or one by name) and the answers. Edits go to the
 * store, so the preview and the quit guard see them before Apply.
 */
export function useMaterialPanel(project: string, model: InspectModel) {
  const asset = model.tweakAsset;
  const saved = useMaterialsStore(s => s.saved[project]) ?? null;
  const edits = useMaterialsStore(s => s.edits[project]);
  const saving = useMaterialsStore(s => s.saving);
  const setLook = useMaterialsStore(s => s.setLook);
  const reset = useMaterialsStore(s => s.reset);
  const apply = useMaterialsStore(s => s.apply);
  const [target, setTarget] = useState(ALL_MATERIALS);
  // A name outside the model's materials falls back to every material.
  const known = target === ALL_MATERIALS || model.materialNames.includes(target);
  const on = known ? target : ALL_MATERIALS;

  const look = asset ? lookOf(saved, edits ?? {}, asset) : emptyLook();
  const tweak = tweakOf(look, on);
  const dirty = Boolean(asset && edits?.[asset]);
  const stored = Boolean(asset && saved?.models[asset]);
  const edit = (patch: Partial<MaterialTweak>) => {
    if (asset) setLook(project, asset, withTweak(look, on, { ...tweak, ...patch }));
  };
  return {
    asset,
    target: on,
    setTarget,
    tweak,
    edit,
    dirty,
    saving,
    /** Some material of the model loses its metal and roughness map. */
    dropsMap: dropsMap(look.all) || Object.values(look.materials).some(dropsMap),
    /** Reset drops the edits; with none, it clears what is saved. */
    resetLabel: dirty ? 'Reset' : 'Clear tweaks',
    canReset: dirty || (stored && !isEmptyLook(look)),
    onReset: () => {
      if (!asset) return;
      if (dirty) reset(project, asset);
      else setLook(project, asset, emptyLook());
    },
    onApply: () => void apply(project),
  };
}

/**
 * A hex field's text: free while typing, committed when it reads as a
 * color, and back to the value when it does not.
 */
export function useHexText(value: string | null, onChange: (hex: string) => void) {
  const [text, setText] = useState(value ?? '');
  useEffect(() => setText(value ?? ''), [value]);
  const commit = () => {
    const hex = parseHex(text);
    if (hex && hex !== value) onChange(hex);
    else setText(value ?? '');
  };
  return { text, setText, commit };
}
