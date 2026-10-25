/**
 * The keys of the app's shortcuts as tooltip keycaps, spelled the way the
 * platform does: Command and Shift glyphs on macOS, words elsewhere.
 */
import { PLATFORM } from '@/ipc/runtime';

export function shortcutKeys(mac: boolean) {
  const mod = mac ? '⌘' : 'Ctrl';
  return {
    /** A chord with Command (macOS) or Ctrl. */
    mod: (key: string): string[] => [mod, key],
    undo: [mod, 'Z'],
    // The editor takes Shift+Mod+Z and Mod+Y; each platform shows its usual one.
    redo: mac ? ['⇧', mod, 'Z'] : [mod, 'Y'],
    save: [mod, 'S'],
    submit: [mod, '↵'],
    escape: 'Esc',
  };
}

export const KEYS = shortcutKeys(PLATFORM === 'mac');
