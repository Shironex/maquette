import { describe, expect, it } from 'vitest';
import { removeRefusal } from './create-project-trash-handlers';

describe('removeRefusal', () => {
  const quiet = { run: null, concepts: 0, sends: 0 };

  it('lets a quiet project go', () => {
    expect(removeRefusal('Moon Island', quiet)).toBeNull();
  });

  it('names what runs, a run first, with plurals', () => {
    expect(removeRefusal('Moon Island', { ...quiet, run: 'run-3', sends: 2 })).toMatch(
      /^A pipeline run is going in Moon Island/
    );
    expect(removeRefusal('Moon Island', { ...quiet, concepts: 1 })).toMatch(
      /^1 concept image is queued/
    );
    expect(removeRefusal('Moon Island', { ...quiet, sends: 2 })).toMatch(/^2 sends are not over/);
  });
});
