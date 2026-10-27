import { describe, expect, it } from 'vitest';
import { underBase } from './base-links';

describe('underBase', () => {
  it('puts root paths under the base', () => {
    expect(underBase('/maquette/', '/docs/start/install/#steps')).toBe(
      '/maquette/docs/start/install/#steps'
    );
    expect(underBase('/maquette', '/icon-64.png')).toBe('/maquette/icon-64.png');
  });

  it('leaves fragments, relative, absolute and already based paths alone', () => {
    for (const href of [
      '#top',
      'next/',
      'https://example.com/',
      '//cdn.example/x',
      '/maquette/docs/',
    ]) {
      expect(underBase('/maquette', href)).toBe(href);
    }
    expect(underBase('/maquette', '/maquette')).toBe('/maquette');
    expect(underBase('/maquette', undefined)).toBe(undefined);
  });

  it('changes nothing at the root base', () => {
    expect(underBase('/', '/docs/')).toBe('/docs/');
  });
});
