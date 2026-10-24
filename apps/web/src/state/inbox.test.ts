import { describe, expect, it } from 'vitest';
import { inboxItems } from '@/ipc/mock/inbox-fixtures';
import { checkSummary, firstToReview, itemChip, itemMeta, reviewPrompt, sortChecks } from './inbox';

const items = inboxItems('/w/inbox', 0);
const byId = (id: string) => items.find(i => i.id === id)!;

describe('inbox helpers', () => {
  it('chips rows by their worst check', () => {
    expect(itemChip(byId('far_spire.glb'))).toEqual({ status: 'failed', label: 'unreadable' });
    expect(itemChip(byId('sakura_large.glb'))).toEqual({ status: 'done', label: 'checks passed' });
    expect(itemChip(byId('character.glb'))).toEqual({ status: 'review', label: 'needs review' });
  });

  it('describes the character the way the real analysis does', () => {
    const character = byId('character.glb');
    expect(itemMeta(character)).toMatch(/^94\.6k tris · \d+ textures?$/);
    expect(character.report!.joints).toBe(65);
    const fingers = character.checks.find(c => c.id === 'finger-weights')!;
    expect(fingers.status).toBe('fail');
    expect(fingers.detail).toMatch(/1,?284/);
  });

  it('puts failures first and summarises', () => {
    const character = byId('character.glb');
    const sorted = sortChecks(character.checks);
    expect(sorted[0]!.status).toBe('fail');
    expect(sorted.at(-1)!.status).toBe('pass');
    expect(checkSummary([])).toBe('No checks');
    expect(checkSummary(byId('sakura_large.glb').checks)).toMatch(/^\d+ passed$/);
    expect(reviewPrompt(character)).toMatch(/^Review the 1 failure/);
    expect(reviewPrompt(byId('far_spire.glb'))).toContain('Reprocess');
    expect(itemMeta(byId('far_spire.glb'))).toContain('not a glTF model');
  });
});

describe('firstToReview', () => {
  it('skips files that could not be read', () => {
    expect(firstToReview(items)!.id).toBe('character.glb');
    expect(firstToReview([byId('far_spire.glb')])!.id).toBe('far_spire.glb');
    expect(firstToReview([])).toBeNull();
  });
});
