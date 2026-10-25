import { describe, expect, it } from 'vitest';
import { parseShipped } from './shipped';

const valid = () => ({
  version: 1,
  project: 'demo',
  models: { lamp: { name: 'Lamp', file: 'lamp_v1.glb', inPlace: true } },
  layout: { version: 1, models: {}, stops: [], desk: null },
  materials: { version: 1, models: {} },
  posters: null,
});

describe('parseShipped', () => {
  it('returns what Ship wrote', () => {
    expect(parseShipped(valid()).models.lamp?.file).toBe('lamp_v1.glb');
  });

  it('refuses a file from a newer Maquette', () => {
    expect(() => parseShipped({ ...valid(), version: 2 })).toThrow(/version 2/);
  });

  it('refuses a model without a .glb file', () => {
    const data = { ...valid(), models: { lamp: { name: 'Lamp', file: 'lamp.png' } } };
    expect(() => parseShipped(data)).toThrow(/lamp has no .glb/);
  });

  it('refuses data without a layout it knows', () => {
    expect(() => parseShipped({ ...valid(), layout: { version: 3 } })).toThrow(/layout/);
    expect(() => parseShipped({ ...valid(), layout: { version: 1, models: {} } })).toThrow(
      /models or stops/
    );
    expect(() => parseShipped('nope')).toThrow(/not an object/);
  });
});
