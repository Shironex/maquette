import { describe, expect, it } from 'vitest';
import { shortcutKeys } from './keys';

describe('shortcutKeys', () => {
  it('uses the Command glyphs on macOS', () => {
    const keys = shortcutKeys(true);
    expect(keys.undo).toEqual(['⌘', 'Z']);
    expect(keys.redo).toEqual(['⇧', '⌘', 'Z']);
    expect(keys.mod('P')).toEqual(['⌘', 'P']);
  });

  it('uses Ctrl elsewhere, with Ctrl+Y for redo', () => {
    const keys = shortcutKeys(false);
    expect(keys.save).toEqual(['Ctrl', 'S']);
    expect(keys.redo).toEqual(['Ctrl', 'Y']);
  });
});
