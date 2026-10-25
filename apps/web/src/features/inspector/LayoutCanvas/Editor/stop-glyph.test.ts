import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { stopGlyph } from './stop-glyph';

const stop = {
  id: 'hero',
  name: 'Hero',
  position: [0, 1, 4] as [number, number, number],
  target: [0, 1, 0] as [number, number, number],
  fov: 40,
  models: null,
};

describe('a camera stop glyph', () => {
  it('stands at the camera and is tagged with the stop', () => {
    const glyph = stopGlyph(stop, '#8fa58c', true, 2);
    expect(glyph.position.toArray()).toEqual([0, 1, 4]);
    expect(glyph.userData.stop).toBe('hero');
    const body = glyph.getObjectByName('stop-body') as THREE.Mesh;
    expect(body.userData.stop).toBe('hero');
    expect((body.material as THREE.MeshBasicMaterial).visible).toBe(false);
  });

  it('puts a dot on what the stop looks at, in its colour', () => {
    const glyph = stopGlyph(stop, '#8fa58c', false, 2);
    const dot = glyph.children.find(
      (o): o is THREE.Mesh => (o as THREE.Mesh).isMesh && o.name !== 'stop-body'
    )!;
    expect(dot.position.toArray()).toEqual([0, 0, -4]);
    expect((dot.material as THREE.MeshBasicMaterial).color.getHexString()).toBe('8fa58c');
  });
});
