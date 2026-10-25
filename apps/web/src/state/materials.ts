/**
 * Material tweaks and per map codecs: the pure rules the Inspector's
 * Material panel, its live preview and the quit guard share.
 */
import type {
  CodecSet,
  FactorMode,
  MapCodecs,
  MaterialTweak,
  ModelMaterials,
  SceneMaterials,
  TextureCodec,
  TextureRow,
} from '@/ipc/client';
import { formatBytes } from '@/lib/format';

/** The Applies to choice for every material at once. */
export const ALL_MATERIALS = '*';
/** The most a roughness multiplier may scale by, as the core checks. */
export const MAX_MULTIPLY = 2;
/** The brightest emissive strength the core accepts. */
export const MAX_EMISSIVE = 50;

/** Tweaks edited in the panel and not applied yet, by asset id. */
export type MaterialEdits = Readonly<Record<string, ModelMaterials>>;

export const emptyTweak = (): MaterialTweak => ({
  tint: null,
  roughness: null,
  metalness: null,
  emissiveStrength: null,
});

export const emptyLook = (): ModelMaterials => ({ all: emptyTweak(), materials: {} });

export const isEmptyTweak = (t: MaterialTweak): boolean =>
  t.tint === null && t.roughness === null && t.metalness === null && t.emissiveStrength === null;

/** The look without empty named entries and with lowercase hex, as the core stores it. */
export function cleanLook(look: ModelMaterials): ModelMaterials {
  const clean = (t: MaterialTweak): MaterialTweak => ({
    ...t,
    tint: t.tint?.toLowerCase() ?? null,
  });
  const materials = Object.fromEntries(
    Object.entries(look.materials)
      .filter(([, t]) => !isEmptyTweak(t))
      .map(([name, t]) => [name, clean(t)])
  );
  return { all: clean(look.all), materials };
}

export const isEmptyLook = (look: ModelMaterials): boolean =>
  isEmptyTweak(look.all) && Object.values(look.materials).every(isEmptyTweak);

export function sameLook(a: ModelMaterials, b: ModelMaterials): boolean {
  return JSON.stringify(cleanLook(a)) === JSON.stringify(cleanLook(b));
}

/** What an asset looks like now: its edit, else what is saved, else nothing. */
export function lookOf(
  saved: SceneMaterials | null,
  edits: MaterialEdits,
  asset: string
): ModelMaterials {
  return edits[asset] ?? saved?.models[asset] ?? emptyLook();
}

/** The edits with `look` in for `asset`; a look back at its saved value is no edit. */
export function editLook(
  saved: SceneMaterials | null,
  edits: MaterialEdits,
  asset: string,
  look: ModelMaterials
): MaterialEdits {
  const { [asset]: _old, ...rest } = edits;
  const was = saved?.models[asset] ?? emptyLook();
  return sameLook(was, look) ? rest : { ...rest, [asset]: look };
}

/** The whole file to save: what is saved with the edits laid over it. */
export function withEdits(saved: SceneMaterials | null, edits: MaterialEdits): SceneMaterials {
  const models: Record<string, ModelMaterials> = { ...(saved?.models ?? {}) };
  for (const [asset, look] of Object.entries(edits)) {
    if (isEmptyLook(look)) delete models[asset];
    else models[asset] = cleanLook(look);
  }
  return { version: saved?.version ?? 1, models };
}

/** The tweak one Applies to choice edits: `all`, or a named material's own fields. */
export function tweakOf(look: ModelMaterials, target: string): MaterialTweak {
  if (target === ALL_MATERIALS) return look.all;
  return look.materials[target] ?? emptyTweak();
}

/** The look with the tweak of `target` replaced. */
export function withTweak(
  look: ModelMaterials,
  target: string,
  tweak: MaterialTweak
): ModelMaterials {
  if (target === ALL_MATERIALS) return { ...look, all: tweak };
  return { ...look, materials: { ...look.materials, [target]: tweak } };
}

/** What a material of that name gets: `all`, with its own fields on top. */
export function tweakFor(look: ModelMaterials, name: string | null | undefined): MaterialTweak {
  const named = name ? look.materials[name] : undefined;
  if (!named) return look.all;
  return {
    tint: named.tint ?? look.all.tint,
    roughness: named.roughness ?? look.all.roughness,
    metalness: named.metalness ?? look.all.metalness,
    emissiveStrength: named.emissiveStrength ?? look.all.emissiveStrength,
  };
}

/** True when the tweak replaces a factor the metal and roughness map scales. */
export function dropsMap(t: MaterialTweak): boolean {
  return t.metalness !== null || t.roughness?.mode === 'override';
}

/** The largest value a roughness of that mode may take. */
export const roughnessMax = (mode: FactorMode): number => (mode === 'multiply' ? MAX_MULTIPLY : 1);

/** A `#rrggbb` string, or null for anything else. */
export function parseHex(text: string): string | null {
  const t = text.trim();
  const hex = t.startsWith('#') ? t : `#${t}`;
  return /^#[0-9a-f]{6}$/i.test(hex) ? hex.toLowerCase() : null;
}

/** The map groups a model picks a codec for. */
export type MapGroup = keyof CodecSet;

export const MAP_GROUPS: readonly { group: MapGroup; label: string; maps: string }[] = [
  { group: 'color', label: 'Color', maps: 'Base color and emissive' },
  { group: 'normal', label: 'Normal', maps: 'Normal maps' },
  { group: 'other', label: 'Data', maps: 'Metal, roughness, occlusion and the rest' },
];

/** The group a texture belongs to, by the slots that use it: color first, then normal. */
export function groupOf(texture: Pick<TextureRow, 'uses'>): MapGroup {
  if (texture.uses.some(u => u === 'base color' || u === 'emissive')) return 'color';
  if (texture.uses.includes('normal')) return 'normal';
  return 'other';
}

export interface GroupSize {
  maps: number;
  bytes: number;
  /** The codecs the built textures carry, `KTX2 UASTC` and so on. */
  built: string[];
}

/** How many maps each group has in a model and what they weigh as built. */
export function groupSizes(textures: readonly TextureRow[]): Record<MapGroup, GroupSize> {
  const out: Record<MapGroup, GroupSize> = {
    color: { maps: 0, bytes: 0, built: [] },
    normal: { maps: 0, bytes: 0, built: [] },
    other: { maps: 0, bytes: 0, built: [] },
  };
  for (const t of textures) {
    const g = out[groupOf(t)];
    g.maps += 1;
    g.bytes += t.bytes ?? 0;
    if (!g.built.includes(t.codec)) g.built.push(t.codec);
  }
  return out;
}

/** The stored per map codecs with one group set. */
export function withGroupCodec(
  stored: MapCodecs | null | undefined,
  group: MapGroup,
  codec: TextureCodec
): MapCodecs {
  return { color: null, normal: null, other: null, ...(stored ?? {}), [group]: codec };
}

/** True when the built textures of a group are not yet in the chosen codec. */
export function builtDiffers(size: GroupSize, codec: TextureCodec): boolean {
  const want = codec === 'uastc' ? 'KTX2 UASTC' : 'KTX2 ETC1S';
  return size.maps > 0 && size.built.some(c => c !== want);
}

const CODEC_NAME: Record<TextureCodec, string> = { etc1s: 'ETC1S', uastc: 'UASTC' };

export interface GroupLine {
  /** `2 maps, 772 KB ETC1S`: the maps as built. */
  text: string;
  /** The built maps are not in the chosen codec yet. */
  due: boolean;
  note: string;
}

/** A group's size line: its maps, bytes and codec as built, and whether a run is due. */
export function groupLine(size: GroupSize, codec: TextureCodec): GroupLine {
  if (size.maps === 0) {
    return { text: 'none in this model', due: false, note: 'This model has no such maps.' };
  }
  const built = size.built.map(c => c.replace(/^KTX2 /, '')).join(' and ');
  const text = `${size.maps} ${size.maps === 1 ? 'map' : 'maps'}, ${formatBytes(size.bytes)} ${built}`;
  const due = builtDiffers(size, codec);
  return {
    text,
    due,
    note: due
      ? `Built as ${built}. Compress encodes them as ${CODEC_NAME[codec]} on the next run.`
      : `Built as ${CODEC_NAME[codec]}, as chosen.`,
  };
}
