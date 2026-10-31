/**
 * Loads a scene's models a few at a time, handing the window back between
 * steps, so a big scene does not stop clicks while it loads. Plain async
 * work, so it runs without a renderer in the tests.
 */
import * as THREE from 'three';

/** Models fetched and parsed at once. */
export const LOAD_AT_ONCE = 3;

/** Lets the window draw and answer clicks before the next step. */
export function nextTask(): Promise<void> {
  return new Promise(resolve => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close();
      resolve();
    };
    channel.port2.postMessage(null);
  });
}

/**
 * Runs `work` over `items`, `limit` at a time, with a turn for the window
 * after each. Starts nothing more once `alive` says no. A failed item does
 * not stop the rest; the answer counts the failures.
 */
export async function inTurns<T>(
  items: readonly T[],
  limit: number,
  alive: () => boolean,
  work: (item: T) => Promise<void>
): Promise<number> {
  let next = 0;
  let failed = 0;
  const worker = async () => {
    while (next < items.length && alive()) {
      const item = items[next]!;
      next += 1;
      try {
        await work(item);
      } catch {
        failed += 1;
      }
      await nextTask();
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return failed;
}

/** The textures under `root`, each once. */
export function texturesOf(root: THREE.Object3D): THREE.Texture[] {
  const found = new Set<THREE.Texture>();
  root.traverse(o => {
    const material = (o as THREE.Mesh).material;
    for (const m of Array.isArray(material) ? material : material ? [material] : []) {
      for (const value of Object.values(m)) {
        if (value instanceof THREE.Texture) found.add(value);
      }
    }
  });
  return [...found];
}

/**
 * Sends a model's textures to the GPU one at a time. Left to the first
 * frame, a whole scene's uploads would land in one long stop.
 */
export async function uploadTextures(
  gl: Pick<THREE.WebGLRenderer, 'initTexture'>,
  root: THREE.Object3D,
  alive: () => boolean
): Promise<void> {
  for (const texture of texturesOf(root)) {
    if (!alive()) return;
    gl.initTexture(texture);
    await nextTask();
  }
}
