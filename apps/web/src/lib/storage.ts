/**
 * Per viewer conveniences only. Storage can be missing or throw (private
 * windows, blocked site data), so every access is guarded.
 */
export function readFlag(key: string): boolean {
  try {
    return window.localStorage.getItem(key) === '1';
  } catch {
    return false;
  }
}

export function writeFlag(key: string, on: boolean): void {
  try {
    if (on) window.localStorage.setItem(key, '1');
    else window.localStorage.removeItem(key);
  } catch {
    // Not remembering is fine.
  }
}
