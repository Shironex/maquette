import { describe, expect, it } from 'vitest';
import { withBase } from './with-base';

describe('withBase', () => {
  const base = import.meta.env.BASE_URL.replace(/\/+$/, '');

  it('puts a root path under the base', () => {
    expect(withBase('/docs/')).toBe(`${base}/docs/`);
    expect(withBase('/#download')).toBe(`${base}/#download`);
  });

  it('takes a path with or without its leading slash', () => {
    expect(withBase('scene/models/a.glb')).toBe(withBase('/scene/models/a.glb'));
  });

  it('gives the base itself for the root', () => {
    expect(withBase('/')).toBe(`${base}/`);
  });
});
