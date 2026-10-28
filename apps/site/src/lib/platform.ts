/**
 * Which build the Download buttons offer first. A visitor on macOS gets the
 * Mac file, one on Windows the Windows file; everyone else (Linux, phones,
 * tablets, a browser that says nothing) gets the neutral Download that leads to
 * both buttons.
 */
export type Platform = 'mac' | 'windows' | 'other';

/** The parts of `navigator` the guess reads. */
export interface NavigatorHints {
  userAgentData?: { platform?: string } | undefined;
  userAgent?: string;
  maxTouchPoints?: number;
}

/**
 * Guesses the platform from `navigator.userAgentData.platform` where the
 * browser has it, else from the user agent string. An iPad asks for the
 * desktop site with a Mac user agent, so a Mac that reports touch points is
 * left as other.
 *
 * The page runs this before its first paint as an inline script built from the
 * function's own source (`platformScript`), so it must stay self-contained:
 * no imports and nothing from the module scope.
 */
export function detectPlatform(nav: NavigatorHints): Platform {
  const hinted = (nav.userAgentData?.platform ?? '').toLowerCase();
  if (hinted === 'macos') return 'mac';
  if (hinted === 'windows') return 'windows';
  if (hinted) return 'other';
  const agent = nav.userAgent ?? '';
  if (/iphone|ipad|ipod|android|windows phone/i.test(agent)) return 'other';
  if (/windows nt|win64|win32/i.test(agent)) return 'windows';
  if (/macintosh|mac os x/i.test(agent)) return (nav.maxTouchPoints ?? 0) > 1 ? 'other' : 'mac';
  return 'other';
}

/**
 * The inline script for the page head: it sets `data-os` on the root element
 * before the first paint, and the stylesheet shows the matching download
 * links (`.os-any`, `.os-mac`, `.os-windows`). Without it the neutral ones show.
 */
export function platformScript(): string {
  return `try{document.documentElement.dataset.os=(${detectPlatform.toString()})(navigator)}catch(e){}`;
}
