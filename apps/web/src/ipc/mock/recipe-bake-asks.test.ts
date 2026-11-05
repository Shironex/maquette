import { describe, expect, it, vi } from 'vitest';
import { record } from '@/test/recipe-bake-fixture';
import { createBackend } from './backend';
import { createRecipeBakeAsks } from './recipe-bake-asks';

describe('recipe bake approvals in mock mode', () => {
  it('keeps one card per recipe until it is added, replaced or denied', () => {
    const changed = vi.fn();
    const asks = createRecipeBakeAsks(changed);
    const id = asks.ask(record());
    expect(asks.ask(record())).toBe(id);
    expect(asks.approvals()).toHaveLength(1);
    expect(asks.approvals()[0]).toMatchObject({
      kind: 'recipe-bake',
      job: 'reach-out',
      assetName: 'Elsheria v3',
    });
    expect(asks.approvals()[0]?.detail).toMatch(/2 numbers of the review warn/);
    asks.close('frostveil', 'reach-out');
    expect(asks.approvals()).toEqual([]);
    const again = asks.ask(record());
    expect(asks.deny('approval-1')).toBe(false);
    expect(asks.deny(again)).toBe(true);
    expect(asks.approvals()).toEqual([]);
    expect(changed).toHaveBeenCalledTimes(4);
  });

  it("has an agent ask to add the demo bake, and the review's add ends it", async () => {
    const handle = createBackend({
      scenario: 'configured',
      probeDelay: 0,
      pace: 0,
      motion: 'demo',
      library: 'demo',
      agents: 'recipe',
    });
    const args = { project: 'frostveil', recipe: 'reach-for-the-lantern' };
    const bake = (await handle('recipe_bake_get', args)) as { review: { warns: number } };
    expect(bake.review.warns).toBe(1);
    const status = (await handle('mcp_status')) as { approvals: { kind: string }[] };
    expect(status.approvals.map(a => a.kind)).toEqual(['recipe-bake']);
    const added = (await handle('recipe_bake_add', args)) as {
      state: string;
      addedVersion: number;
    };
    expect(added).toMatchObject({ state: 'added', addedVersion: 3 });
    const after = (await handle('mcp_status')) as { approvals: unknown[] };
    expect(after.approvals).toEqual([]);
    await expect(handle('recipe_bake_add', args)).rejects.toMatchObject({
      message: 'This bake was added already, as v3.',
    });
  });
});
