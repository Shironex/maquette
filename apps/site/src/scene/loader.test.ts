import type * as THREE from 'three';
import { describe, expect, it, vi } from 'vitest';
import { loadInOrder } from './loader';

describe('loadInOrder', () => {
  it('starts nothing and reports nothing once torn down', async () => {
    const onLoaded = vi.fn();
    const onError = vi.fn();
    const renderer = {} as THREE.WebGLRenderer;
    await loadInOrder(renderer, ['a.glb', 'b.glb'], onLoaded, onError, () => false);
    expect(onLoaded).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });
});
