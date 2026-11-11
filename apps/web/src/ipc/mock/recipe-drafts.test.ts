import { describe, expect, it } from 'vitest';
import type { RecipeDoc } from '../client';
import { createBackend } from './backend';
import { POSE_KEYS_DEMO } from './demo-pose-keys';

function backend(agents: string | null) {
  return createBackend({
    scenario: 'configured',
    probeDelay: 0,
    pace: 0,
    library: 'demo',
    agents,
    emit: () => {},
  });
}

const read = async (handle: ReturnType<typeof backend>) =>
  (await handle('recipe_get', { project: 'frostveil', id: POSE_KEYS_DEMO })) as RecipeDoc;

describe('draft marks in mock mode', () => {
  it('carries an agent keys on the pose keys demo until a person saves', async () => {
    const handle = backend('drafts');
    const doc = await read(handle);
    expect(doc.drafts).toMatchObject({ by: 'Motion agent', layers: ['l-1'] });
    expect(doc.drafts?.keys).toHaveLength(2);

    // A key the recipe drops loses its mark; the rest stay.
    const layer = doc.recipe.layers[0]!;
    const kept = { ...layer, keys: (layer.keys ?? []).filter(k => k.frame !== 20) };
    const saved = (await handle('recipe_update', {
      project: 'frostveil',
      id: POSE_KEYS_DEMO,
      change: { layers: [kept], expected: doc.version },
    })) as RecipeDoc;
    expect(saved.drafts).toBeUndefined();
    expect((await read(handle)).drafts).toBeUndefined();
  });

  it('has none without the scene', async () => {
    expect((await read(backend(null))).drafts).toBeUndefined();
  });
});
