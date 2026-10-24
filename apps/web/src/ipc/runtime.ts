/**
 * Facts about where the UI runs, decided once before anything else imports
 * them. Mock mode installs a fake `__TAURI_INTERNALS__`, so asking "is this
 * Tauri?" after that point would lie; read these constants instead.
 */
import type { ChromeMode } from '@maquette/ui';

export type Platform = 'mac' | 'windows' | 'linux';

function detectPlatform(): Platform {
  const agent = typeof navigator === 'undefined' ? '' : navigator.userAgent;
  if (/Windows/i.test(agent)) return 'windows';
  if (/Mac OS X|Macintosh/i.test(agent)) return 'mac';
  return 'linux';
}

/** True inside the Tauri webview: it injects this global before any page script runs. */
export const IN_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const PLATFORM: Platform = detectPlatform();

/**
 * macOS draws its own lights over the web view (overlay title bar); Windows and
 * Linux keep the native title bar; a plain browser gets the kit's drawn lights.
 */
export const CHROME: ChromeMode = !IN_TAURI ? 'drawn' : PLATFORM === 'mac' ? 'overlay' : 'native';
