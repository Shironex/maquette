import { describe, expect, it } from 'vitest';
import { inboxItems } from '@/ipc/mock/inbox-fixtures';
import {
  acceptTarget,
  checkSummary,
  firstToReview,
  goesTo,
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
    expect(itemChip(byId('rune_obelisk.glb'))).toEqual({ status: 'failed', label: 'unreadable' });
    expect(itemChip(byId('veil_gate.glb'))).toEqual({ status: 'done', label: 'checks passed' });
    expect(itemChip(byId('elsheria.glb'))).toEqual({ status: 'review', label: 'needs review' });
  });

  it('describes the character the way the real analysis does', () => {
    const character = byId('elsheria.glb');
    expect(itemMeta(character)).toMatch(/^115\.7k tris · \d+ textures?$/);
    expect(character.report!.joints).toBe(119);
    const fingers = character.checks.find(c => c.id === 'finger-weights')!;
    expect(fingers.status).toBe('fail');
    expect(fingers.detail).toMatch(/1,?284/);
  });

  it('puts failures first and summarises', () => {
    const character = byId('elsheria.glb');
    const sorted = sortChecks(character.checks);
    expect(sorted[0]!.status).toBe('fail');
    expect(sorted.at(-1)!.status).toBe('pass');
    expect(checkSummary([])).toBe('No checks');
    expect(checkSummary(byId('veil_gate.glb').checks)).toMatch(/^\d+ passed$/);
    expect(reviewPrompt(character)).toMatch(/^Review the 1 failure/);
    expect(reviewPrompt(byId('rune_obelisk.glb'))).toContain('Reprocess');
    expect(itemMeta(byId('rune_obelisk.glb'))).toContain('not a glTF model');
  });
});

describe('goesTo', () => {
  it('sends an attached download to the asset it is aimed at', () => {
    const projects = [
      { id: 'frostveil', name: 'Frostveil' },
      { id: 'academy', name: 'Academy' },
    ];
    const free = byId('veil_gate.glb');
    expect(goesTo(free)).toBeNull();
    expect(acceptTarget(free, projects, projects[0]!)).toBe(projects[0]);
    const aim = { project: 'academy', asset: 'sigil-door', assetName: 'Sigil door' };
    const aimed = { ...free, aim };
    expect(goesTo(aimed)).toEqual(aim);
    expect(acceptTarget(aimed, projects, projects[0]!)).toBe(projects[1]);
  });
});

describe('firstToReview', () => {
  it('skips files that could not be read', () => {
    expect(firstToReview(items)!.id).toBe('elsheria.glb');
    expect(firstToReview([byId('rune_obelisk.glb')])!.id).toBe('rune_obelisk.glb');
    expect(firstToReview([])).toBeNull();
  });
});

describe('madeBy and itemTitle', () => {
  const base = byId('elsheria.glb');
  const made = (generator: string | null) =>
    madeBy({ ...base, link: null, report: { ...base.report!, generator } });
  const link = {
    job: 'job-1',
    project: 'frostveil',
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
    const shrubs = byId('ice_crystals.glb');
    expect(shrubs.kindFromSend).toBe(true);
    expect(kindHint(shrubs)).toBe('From the send. Pick one to change the checks.');
    expect(kindHint(byId('elsheria.glb'))).toBe(
      'A guess from the file. Pick one to change the checks.'
    );
    expect(kindHint({ kindGuessed: false, kindFromSend: false })).toBeUndefined();
  });

  it('shows a send without PBR as a passing row, not a warning', () => {
    const shrubs = byId('ice_crystals.glb');
    expect(shrubs.checks.find(c => c.id === 'pbr')).toMatchObject({
      status: 'pass',
      title: 'PBR off, as sent',
    });
    expect(reviewPrompt(shrubs)).toContain('Review the 3 warnings');
  });
});
