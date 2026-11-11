import { describe, expect, it } from 'vitest';

import { seedInbox } from './triage-fixtures';

describe('?inbox=walk', () => {
  it('adds a character whose walk goes against its face, flagged as the core flags it', () => {
    const items = seedInbox('walk', '/ws/inbox', 0);
    const walker = items.find(i => i.name === 'Ash walker');
    expect(walker?.kind).toBe('character');
    expect(walker?.report?.clipTravel?.[0]?.clip).toBe('Walk');
    const row = walker?.checks.find(c => c.id === 'walk-facing');
    expect(row?.status).toBe('warning');
    expect(row?.title).toBe('Walk travels against the face');
    expect(walker?.checks.find(c => c.id === 'animations')?.title).toBe('2 animations');
    // The usual models stay.
    expect(items.length).toBe(seedInbox(null, '/ws/inbox', 0).length + 1);
  });
});
