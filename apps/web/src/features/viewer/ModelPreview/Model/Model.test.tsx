import { renderHook } from '@testing-library/react';
import * as THREE from 'three';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { originalsOf } from '../../materials';
import type { Loaded } from '../ModelPreview.types';
import Model from './Model';

function loadedBox(): { loaded: Loaded; mesh: THREE.Mesh; own: THREE.Material } {
  const own = new THREE.MeshStandardMaterial();
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(), own);
  const scene = new THREE.Group();
  scene.add(mesh);
  return { loaded: { scene, originals: originalsOf(scene), bones: [] }, mesh, own };
}

beforeEach(() => {
  // jsdom has no 2D canvas; the UV checker stays blank.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});
afterEach(() => vi.restoreAllMocks());

describe('the previewed model', () => {
  it('draws the loaded scene', () => {
    const { loaded } = loadedBox();
    const { result } = renderHook(() => Model({ loaded, mode: 'shaded' }));
    expect(result.current.type).toBe('primitive');
    expect(result.current.props).toEqual({ object: loaded.scene });
  });

  it('wears the look of the mode and its own materials again after', () => {
    const { loaded, mesh, own } = loadedBox();
    const { rerender } = renderHook(({ mode }) => Model({ loaded, mode }), {
      initialProps: { mode: 'uv' as 'uv' | 'shaded' },
    });
    expect(mesh.material).not.toBe(own);
    rerender({ mode: 'shaded' });
    expect(mesh.material).toBe(own);
  });
});
