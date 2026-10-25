/**
 * Material tweaks in mock mode: each project keeps its `materials.json` in
 * memory, and a save is checked and tidied the way the core does it.
 */
import type { MaterialTweak, ModelMaterials, SceneMaterials } from '../client';
import { fail, type Args } from './backend-options';

const MAX_NAMED = 64;
const NAME_MAX_CHARS = 120;

function inRange(what: string, value: number | null, max: number): void {
  if (value === null) return;
  if (!Number.isFinite(value) || value < 0 || value > max) {
    fail('invalid', `${what} must be between 0 and ${max}.`);
  }
}

function check(asset: string, name: string, t: MaterialTweak): MaterialTweak {
  const where = `${asset} (${name})`;
  if (t.tint !== null && !/^#[0-9a-f]{6}$/i.test(t.tint)) {
    fail('invalid', `${where}: the tint must be a #rrggbb color.`);
  }
  if (t.roughness) {
    inRange(`${where}: roughness`, t.roughness.value, t.roughness.mode === 'multiply' ? 2 : 1);
  }
  inRange(`${where}: metalness`, t.metalness, 1);
  inRange(`${where}: emissive strength`, t.emissiveStrength, 50);
  return { ...t, tint: t.tint?.toLowerCase() ?? null };
}

const isEmpty = (t: MaterialTweak) =>
  t.tint === null && t.roughness === null && t.metalness === null && t.emissiveStrength === null;

/** The stored file: checked, lowercase tints, no empty entries or models. */
export function tidyMaterials(next: SceneMaterials): SceneMaterials {
  const models: Record<string, ModelMaterials> = {};
  for (const [asset, look] of Object.entries(next.models)) {
    const named = Object.entries(look.materials);
    if (named.length > MAX_NAMED)
      fail('invalid', `${asset} names more than ${MAX_NAMED} materials.`);
    const materials = Object.fromEntries(
      named
        .map(([name, t]) => {
          if (!name.trim() || name.length > NAME_MAX_CHARS) {
            fail('invalid', `${asset} has a material name that is empty or too long.`);
          }
          return [name, check(asset, name, t)] as const;
        })
        .filter(([, t]) => !isEmpty(t))
    );
    const all = check(asset, 'all materials', look.all);
    if (!isEmpty(all) || Object.keys(materials).length) models[asset] = { all, materials };
  }
  return { version: 1, models };
}

/** The `materials_get` and `materials_save` commands. */
export function createMaterialHandlers(
  project: (id: string) => unknown
): Record<string, (args: Args) => unknown> {
  const stored = new Map<string, SceneMaterials>();
  const get = (id: string) => stored.get(id) ?? { version: 1, models: {} };
  return {
    materials_get: ({ project: id }) => {
      project(String(id));
      return get(String(id));
    },
    materials_save: ({ project: id, materials }) => {
      project(String(id));
      const tidy = tidyMaterials(materials as SceneMaterials);
      stored.set(String(id), tidy);
      return tidy;
    },
  };
}
