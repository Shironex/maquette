import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { buildTheme, themeNames } from '../../scripts/theme.mjs';
import { THEME_NAMES } from '../lib/theme-names';

const root = resolve(__dirname, '../..');
const tokens = JSON.parse(readFileSync(resolve(root, 'tokens.json'), 'utf8'));

describe('theme', () => {
  it('matches what tokens.json generates (run gen:theme after editing tokens)', () => {
    const onDisk = readFileSync(resolve(root, 'src/styles/theme.css'), 'utf8');
    expect(onDisk).toBe(buildTheme(tokens));
  });

  it('keeps the tailwind-merge name list in step with the tokens', () => {
    expect(THEME_NAMES).toEqual(themeNames(tokens));
  });

  it('carries every kit colour token as a raw custom property', () => {
    const css = buildTheme(tokens);
    for (const t of tokens.color.tokens) expect(css).toContain(`--${t.name}:`);
    expect(css).toContain('--clay-text: #dc9a7e;');
  });
});
