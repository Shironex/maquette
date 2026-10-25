import { describe, expect, it } from 'vitest';
import { inboxItems } from '@/ipc/mock/inbox-fixtures';
import {
  checkSummary,
  firstToReview,
  itemChip,
  itemMeta,
  itemTitle,
  kindHint,
  madeBy,
  reviewPrompt,
  sortChecks,
} from './inbox';

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

describe('madeBy and itemTitle', () => {
  const base = byId('character.glb');
  const made = (generator: string | null) =>
    madeBy({ ...base, link: null, report: { ...base.report!, generator } });
  const link = {
    job: 'job-1',
    project: 'moon-island',
    asset: 'clay-lantern',
    assetName: 'Clay lantern',
    provider: 'meshy' as const,
  };

  it('names known generators instead of showing their URL', () => {
    expect(made('https://tripo3d.ai')).toBe('Tripo');
    expect(made('https://www.meshy.ai/')).toBe('Meshy');
    expect(made('Khronos glTF Blender I/O v4.3.47')).toBe('Blender');
    expect(made('https://www.example.com/exporter?v=2')).toBe('example.com');
    expect(made('Sketchfab exporter')).toBe('Sketchfab exporter');
    expect(made(null)).toBeNull();
  });

  it('prefers the provider a model was sent to', () => {
    const sent = { ...base, link, report: { ...base.report!, generator: 'https://tripo3d.ai' } };
    expect(madeBy(sent)).toBe('Meshy');
    expect(madeBy({ ...sent, link: { ...link, provider: 'by-hand' } })).toBe('Tripo');
  });

  it('titles a sent model after its asset, not its file', () => {
    const clay = { ...base, id: 'clay-1.glb', name: 'Clay 1' };
    expect(itemTitle(clay)).toBe('Clay 1');
    expect(itemTitle({ ...clay, link })).toBe('Clay lantern');
  });
});

describe('kindHint', () => {
  it('says whether the kind came from the send, a guess, or a pick', () => {
    const shrubs = byId('flower_shrubs.glb');
    expect(shrubs.kindFromSend).toBe(true);
    expect(kindHint(shrubs)).toBe('From the send. Pick one to change the checks.');
    expect(kindHint(byId('character.glb'))).toBe(
      'A guess from the file. Pick one to change the checks.'
    );
    expect(kindHint({ kindGuessed: false, kindFromSend: false })).toBeUndefined();
  });

  it('shows a send without PBR as a passing row, not a warning', () => {
    const shrubs = byId('flower_shrubs.glb');
    expect(shrubs.checks.find(c => c.id === 'pbr')).toMatchObject({
      status: 'pass',
      title: 'PBR off, as sent',
    });
    expect(reviewPrompt(shrubs)).toContain('Review the 3 warnings');
  });
});
