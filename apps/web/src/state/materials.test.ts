import { describe, expect, it } from 'vitest';
import type { MaterialTweak, ModelMaterials, TextureRow } from '@/ipc/client';
import {
  ALL_MATERIALS,
  builtDiffers,
  cleanLook,
  dropsMap,
  editLook,
  emptyLook,
  emptyTweak,
  groupLine,
  groupOf,
  groupSizes,
  lookOf,
  parseHex,
  tweakFor,
  tweakOf,
  withEdits,
  withGroupCodec,
  withTweak,
} from './materials';

const tweak = (over: Partial<MaterialTweak>): MaterialTweak => ({ ...emptyTweak(), ...over });
const look = (all: Partial<MaterialTweak>, materials: Record<string, MaterialTweak> = {}) =>
  ({ all: tweak(all), materials }) satisfies ModelMaterials;
const saved = { version: 1, models: { crate: look({ tint: '#aa0000' }) } };

describe('material looks', () => {
  it('reads an edit first, then what is saved, then nothing', () => {
    expect(lookOf(saved, {}, 'crate').all.tint).toBe('#aa0000');
    expect(lookOf(saved, { crate: look({ tint: '#00aa00' }) }, 'crate').all.tint).toBe('#00aa00');
    expect(lookOf(saved, {}, 'lamp')).toEqual(emptyLook());
  });

  it('drops an edit that goes back to the saved look', () => {
    const edits = editLook(saved, {}, 'crate', look({ tint: '#00aa00' }));
    expect(Object.keys(edits)).toEqual(['crate']);
    // Uppercase hex reads as the same color the core stores.
    expect(editLook(saved, edits, 'crate', look({ tint: '#AA0000' }))).toEqual({});
    expect(editLook(null, {}, 'lamp', emptyLook())).toEqual({});
  });

  it('lays edits over the file, tidied, and removes emptied assets', () => {
    const named = { Lid: tweak({ metalness: 0.2 }), Base: emptyTweak() };
    const file = withEdits(saved, {
      crate: emptyLook(),
      lamp: look({ tint: '#FFEE00' }, named),
    });
    expect(file).toEqual({
      version: 1,
      models: { lamp: look({ tint: '#ffee00' }, { Lid: tweak({ metalness: 0.2 }) }) },
    });
    expect(cleanLook(look({}, { Base: emptyTweak() })).materials).toEqual({});
  });

  it('edits every material or one by name, and a name gets all under its own fields', () => {
    const base = look({ tint: '#808080', metalness: 0 });
    const lid = withTweak(base, 'Lid', tweak({ metalness: 0.9 }));
    expect(tweakOf(lid, ALL_MATERIALS)).toEqual(base.all);
    expect(tweakOf(lid, 'Lid')).toEqual(tweak({ metalness: 0.9 }));
    expect(tweakOf(lid, 'Base')).toEqual(emptyTweak());
    expect(tweakFor(lid, 'Lid')).toEqual(tweak({ tint: '#808080', metalness: 0.9 }));
    expect(tweakFor(lid, 'Base')).toEqual(base.all);
    expect(tweakFor(lid, null)).toEqual(base.all);
  });

  it('knows which tweaks replace the metal and roughness map', () => {
    expect(dropsMap(tweak({ metalness: 0 }))).toBe(true);
    expect(dropsMap(tweak({ roughness: { mode: 'override', value: 0.8 } }))).toBe(true);
    expect(dropsMap(tweak({ roughness: { mode: 'multiply', value: 1.2 } }))).toBe(false);
    expect(dropsMap(tweak({ tint: '#ffffff' }))).toBe(false);
  });

  it('reads hex colors with or without the hash', () => {
    expect(parseHex('6F7446')).toBe('#6f7446');
    expect(parseHex(' #3a3936 ')).toBe('#3a3936');
    expect(parseHex('#fff')).toBeNull();
    expect(parseHex('olive')).toBeNull();
  });
});

const row = (uses: string[], bytes: number, codec: string): TextureRow => ({
  name: uses.join('+'),
  uses,
  width: 1024,
  height: 1024,
  bytes,
  codec,
  gpuBytes: null,
});

describe('map groups', () => {
  const textures = [
    row(['base color'], 600_000, 'KTX2 ETC1S'),
    row(['emissive'], 40_000, 'KTX2 ETC1S'),
    row(['normal'], 400_000, 'KTX2 UASTC'),
    row(['metal and roughness', 'occlusion'], 200_000, 'KTX2 ETC1S'),
  ];

  it('sorts maps by the slots that use them', () => {
    expect(groupOf(row(['base color', 'normal'], 0, 'PNG'))).toBe('color');
    expect(groupOf(row(['normal'], 0, 'PNG'))).toBe('normal');
    expect(groupOf(row(['occlusion'], 0, 'PNG'))).toBe('other');
  });

  it('sums what each group weighs as built and flags a group a run would change', () => {
    const sizes = groupSizes(textures);
    expect(sizes.color).toEqual({ maps: 2, bytes: 640_000, built: ['KTX2 ETC1S'] });
    expect(sizes.normal.bytes).toBe(400_000);
    expect(builtDiffers(sizes.normal, 'uastc')).toBe(false);
    expect(builtDiffers(sizes.normal, 'etc1s')).toBe(true);
    expect(builtDiffers(groupSizes([]).normal, 'etc1s')).toBe(false);

    expect(groupLine(sizes.color, 'etc1s')).toMatchObject({
      text: '2 maps, 640 KB ETC1S',
      due: false,
    });
    const due = groupLine(sizes.other, 'uastc');
    expect(due.due).toBe(true);
    expect(due.note).toMatch(/Compress encodes them as UASTC/);
    expect(groupLine(groupSizes([]).normal, 'uastc').text).toBe('none in this model');
  });

  it('sets one group and leaves the others to the row codec', () => {
    expect(withGroupCodec(null, 'normal', 'etc1s')).toEqual({
      color: null,
      normal: 'etc1s',
      other: null,
    });
    expect(withGroupCodec({ color: 'uastc', normal: null, other: null }, 'other', 'etc1s')).toEqual(
      { color: 'uastc', normal: null, other: 'etc1s' }
    );
  });
});
