import { describe, expect, it } from 'vitest';
import { fileManagerName, revealLabel } from './file-manager';

describe('the file manager words', () => {
  it('names the file manager of each OS', () => {
    expect(fileManagerName('mac')).toBe('Finder');
    expect(fileManagerName('windows')).toBe('Explorer');
    expect(fileManagerName('linux')).toBe('the file manager');
    expect(fileManagerName(undefined)).toBe('Finder');
  });

  it('reveals on a Mac and shows elsewhere', () => {
    expect(revealLabel('mac', 'image')).toBe('Reveal image in Finder');
    expect(revealLabel('windows', 'image')).toBe('Show image in Explorer');
    expect(revealLabel('linux', 'image')).toBe('Show image in the file manager');
  });
});
