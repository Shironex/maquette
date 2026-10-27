import { describe, expect, it } from 'vitest';
import { gameProject } from './game-fixtures';
import { linkGameInbox, withFolderBudget } from './game-inbox';
import { inboxItems } from './inbox-fixtures';

const items = () => inboxItems('/w/inbox', 0);
const props = { folder: 'props', triangles: 12_000, texture: 1024 };

describe('game inbox in mock mode', () => {
  it('checks a model sent for a game asset against its folder budget', () => {
    const shrubs = items().find(i => i.id === 'flower_shrubs.glb')!;
    expect(shrubs.checks.find(c => c.id === 'triangles')?.status).toBe('pass');
    const checked = withFolderBudget(shrubs, props);
    const tris = checked.checks.find(c => c.id === 'triangles')!;
    expect(tris.status).toBe('fail');
    expect(tris.detail).toMatch(/^Over the props budget, 12k tris\./);
    expect(tris.fixes).toEqual([{ op: 'decimate', triangles: 12_000 }]);
    const tex = checked.checks.find(c => c.id === 'textures')!;
    expect(tex.detail).toContain('over the props budget, 1024 px');
    expect(checked.worst).toBe('fail');
    const roomy = withFolderBudget(shrubs, { folder: '', triangles: 40_000, texture: 4096 });
    expect(roomy.checks.find(c => c.id === 'triangles')?.detail).toBe(
      "Within the model folder's budget, 40k tris."
    );
  });

  it('links the shrubs to Candle cluster in the crypt game and leaves a website alone', () => {
    const crypt = gameProject('crypt', true);
    crypt.assets = [
      ...(crypt.assets ?? []),
      { ...crypt.assets![0]!, id: 'candle-cluster', name: 'Candle cluster', kind: 'prop' },
    ];
    delete crypt.game!.models['candle-cluster'];
    const linked = linkGameInbox(items(), crypt).find(i => i.id === 'flower_shrubs.glb')!;
    expect(linked.link?.asset).toBe('candle-cluster');
    expect(linked.checks.find(c => c.id === 'triangles')?.detail).toContain('props budget');
    expect(linkGameInbox(items(), null)).toEqual(items());
  });
});
