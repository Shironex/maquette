import { describe, expect, it } from 'vitest';
import { absoluteUrl } from './site-url';

describe('absoluteUrl', () => {
  it('keeps the path of the site', () => {
    expect(absoluteUrl('/docs/a.md', 'https://example.test/maquette')).toBe(
      'https://example.test/maquette/docs/a.md'
    );
  });

  it('joins with one slash whatever the ends look like', () => {
    expect(absoluteUrl('docs/a.md', 'https://example.test/maquette/')).toBe(
      'https://example.test/maquette/docs/a.md'
    );
  });
});
