import * as THREE from 'three';
import { describe, expect, it, vi } from 'vitest';
import { inTurns, nextTask, texturesOf, uploadTextures } from './load-in-turns';

const gate = () => {
  let open = () => {};
  const opened = new Promise<void>(resolve => {
    open = resolve;
  });
  return { open, opened };
};

function textured(): { root: THREE.Group; map: THREE.Texture } {
  const map = new THREE.Texture();
  const material = new THREE.MeshStandardMaterial({ map, roughnessMap: map });
  const root = new THREE.Group();
  root.add(new THREE.Mesh(new THREE.BoxGeometry(), material));
  return { root, map };
}

describe('inTurns', () => {
  it('runs no more than the limit at once and finishes them all', async () => {
    let now = 0;
    let most = 0;
    const seen: number[] = [];
    await inTurns(
      [1, 2, 3, 4, 5, 6, 7],
      3,
      () => true,
      async item => {
        now += 1;
        most = Math.max(most, now);
        await nextTask();
        seen.push(item);
        now -= 1;
      }
    );
    expect(most).toBe(3);
    expect(seen.toSorted()).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it('gives the window a turn between items', async () => {
    let ticks = 0;
    const timer = setInterval(() => {
      ticks += 1;
    }, 0);
    await inTurns(
      Array.from({ length: 40 }, (_, i) => i),
      1,
      () => true,
      async () => {
        // Busy work with no await of its own: only inTurns can hand the window back.
        const until = performance.now() + 2;
        while (performance.now() < until);
      }
    );
    clearInterval(timer);
    expect(ticks).toBeGreaterThan(5);
  });

  it('starts nothing more once it is no longer wanted', async () => {
    const first = gate();
    const started: number[] = [];
    let wanted = true;
    const done = inTurns(
      [1, 2, 3, 4, 5],
      2,
      () => wanted,
      async item => {
        started.push(item);
        await first.opened;
      }
    );
    await nextTask();
    wanted = false;
    first.open();
    await done;
    expect(started).toEqual([1, 2]);
  });

  it('counts the failures and carries on', async () => {
    const work = vi.fn(async (item: number) => {
      if (item % 2) throw new Error('broken');
    });
    expect(await inTurns([1, 2, 3, 4], 2, () => true, work)).toBe(2);
    expect(work).toHaveBeenCalledTimes(4);
  });
});

describe('uploadTextures', () => {
  it('finds a shared texture once', () => {
    const { root, map } = textured();
    expect(texturesOf(root)).toEqual([map]);
  });

  it('sends each texture and stops when no longer wanted', async () => {
    const { root, map } = textured();
    const initTexture = vi.fn();
    await uploadTextures({ initTexture }, root, () => true);
    expect(initTexture).toHaveBeenCalledExactlyOnceWith(map);
    initTexture.mockClear();
    await uploadTextures({ initTexture }, root, () => false);
    expect(initTexture).not.toHaveBeenCalled();
  });
});
