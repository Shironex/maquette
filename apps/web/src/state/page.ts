/**
 * The Page view's pure parts: the device presets and how a frame fits its
 * box, the address a frame loads, the page bridge's messages (see the root
 * README, Page view), the scrubber's stops and line, and what the frame
 * shows for the dev server's state. The numbers are in ./page-numbers.ts.
 */
import type { IconName } from '@maquette/ui';
import type { PageView, ServerStatus } from '@/ipc/client';
import { fileUrl, frameUrl } from '@/ipc/files';
import { formatCount } from '@/lib/format';

export type DeviceId = 'phone' | 'laptop' | 'desktop';

export interface Device {
  id: DeviceId;
  label: string;
  icon: IconName;
  /** CSS pixels the page lays out at. */
  width: number;
  height: number;
}

export const DEVICES: readonly Device[] = [
  { id: 'phone', label: 'Phone', icon: 'device-mobile', width: 390, height: 844 },
  { id: 'laptop', label: 'Laptop', icon: 'laptop', width: 1280, height: 800 },
  { id: 'desktop', label: 'Desktop', icon: 'desktop', width: 1440, height: 900 },
];

export function device(id: DeviceId): Device {
  return DEVICES.find(d => d.id === id) ?? DEVICES[2]!;
}

/** The scale that fits the device's viewport into the box, never above 1. */
export function fitScale(d: Device, box: { width: number; height: number }): number {
  if (box.width <= 0 || box.height <= 0) return 0;
  return Math.min(1, box.width / d.width, box.height / d.height);
}

/** "Desktop 1440 × 900 · shown at 58%". */
export function fitLabel(d: Device, scale: number): string {
  const shown = scale >= 1 ? 'shown at full size' : `shown at ${Math.round(scale * 100)}%`;
  return `${d.label} ${d.width} × ${d.height} · ${shown}`;
}

/** Where a frame points, or null while there is nothing to show. */
export function pageAddress(
  base: string | null,
  path: string,
  options: { poster?: boolean } = {}
): string | null {
  if (!base) return null;
  let url: URL;
  try {
    url = new URL(base);
  } catch {
    return null;
  }
  // Frames load localhost names; [::1] reads the same and passes the CSP.
  if (url.hostname === '[::1]') url.hostname = 'localhost';
  if (url.protocol === 'http:' || url.protocol === 'https:') {
    url.pathname = path.startsWith('/') ? path : '/';
  }
  if (options.poster) url.searchParams.set('maquette-poster', '1');
  return url.href;
}

/** "localhost:4321/pricing": the address bar, without the scheme. */
export function shortAddress(url: string | null): string {
  if (!url) return '';
  try {
    const u = new URL(url);
    if (u.protocol !== 'http:' && u.protocol !== 'https:') {
      return decodeURIComponent(u.pathname.split('/').pop() ?? url);
    }
    return `${u.host}${u.pathname}`;
  } catch {
    return url;
  }
}

/** The site's folder name, for the path button: "moon-island-site". */
export function siteName(repo: string | null): string {
  if (!repo) return 'No site';
  return (
    repo
      .replace(/[\\/]+$/, '')
      .split(/[\\/]/)
      .pop() ?? repo
  );
}

// The page bridge.

export const BRIDGE_VERSION = 1;

export interface BridgeStop {
  name: string;
}

/**
 * Who answers: the site's own bridge, or Maquette's shim, which the frame
 * proxy adds to a page without one (it says `source: 'shim'`).
 */
export type BridgeSource = 'site' | 'shim';

export type FromPage = { source: BridgeSource } & (
  | { type: 'ready'; stops: BridgeStop[]; features: string[] }
  | { type: 'at'; stop: number; scroll: { top: number; max: number } | null }
  | { type: 'stats'; frameMs: number; p95Ms: number; drawCalls: number | null }
);

export type ToPage =
  | { maquette: 1; type: 'hello'; stops: BridgeStop[] }
  | { maquette: 1; type: 'goto'; stop: number; smooth: boolean };

/** Hello carries the project's camera stops: the shim has no stops of its own to say. */
export const hello = (stops: readonly BridgeStop[] = []): ToPage => ({
  maquette: 1,
  type: 'hello',
  stops: stops.map(s => ({ name: s.name })),
});

export const goto = (stop: number, smooth = false): ToPage => ({
  maquette: 1,
  type: 'goto',
  stop: Math.round(stop * 1000) / 1000,
  smooth,
});

const isNum = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

function readScroll(v: unknown): { top: number; max: number } | null {
  if (typeof v !== 'object' || v === null) return null;
  const s = v as Record<string, unknown>;
  return isNum(s.top) && isNum(s.max) ? { top: s.top, max: s.max } : null;
}

/** A message from the page, or null for anything else posted to the window. */
export function readPageMessage(data: unknown): FromPage | null {
  if (typeof data !== 'object' || data === null) return null;
  const m = data as Record<string, unknown>;
  if (m.maquette !== BRIDGE_VERSION) return null;
  const source: BridgeSource = m.source === 'shim' ? 'shim' : 'site';
  if (m.type === 'ready' && Array.isArray(m.stops)) {
    const stops = m.stops
      .map(s => (typeof s === 'object' && s !== null ? (s as Record<string, unknown>).name : null))
      .filter((n): n is string => typeof n === 'string' && n.trim() !== '')
      .slice(0, 50)
      .map(name => ({ name: name.slice(0, 60) }));
    const features = Array.isArray(m.features)
      ? m.features.filter((f): f is string => typeof f === 'string')
      : [];
    return { source, type: 'ready', stops, features };
  }
  if (m.type === 'at' && isNum(m.stop)) {
    return { source, type: 'at', stop: Math.max(0, m.stop), scroll: readScroll(m.scroll) };
  }
  if (m.type === 'stats' && isNum(m.frameMs) && isNum(m.p95Ms)) {
    return {
      source,
      type: 'stats',
      frameMs: m.frameMs,
      p95Ms: m.p95Ms,
      drawCalls: isNum(m.drawCalls) ? Math.round(m.drawCalls) : null,
    };
  }
  return null;
}

// The scrubber.

/** The nearest stop to fractional progress `at`. */
export function nearestStop(at: number, count: number): number {
  if (count <= 0) return 0;
  return Math.min(count - 1, Math.max(0, Math.round(at)));
}

/** Progress as a share of the tour, 0 to 1. */
export function share(at: number, count: number): number {
  if (count <= 1) return 0;
  return Math.min(1, Math.max(0, at / (count - 1)));
}

/** Where a stop's label may sit on the scrubber, as shares of its width. */
export interface StopSlot {
  /** Where its mark is. */
  left: number;
  /** How much room its label has. */
  width: number;
  /** The last stop: its label ends at its mark instead of starting there. */
  end: boolean;
}

/**
 * The room each stop's label has: from its mark to the next one, except
 * that the last two split the last gap, since the last label reads leftward
 * from the end. So no label ever runs into its neighbour, however many stops.
 */
export function stopSlots(count: number): StopSlot[] {
  if (count <= 0) return [];
  if (count === 1) return [{ left: 0, width: 1, end: false }];
  const gap = 1 / (count - 1);
  return Array.from({ length: count }, (_, i) => ({
    left: i * gap,
    width: i >= count - 2 ? gap / 2 : gap,
    end: i === count - 1,
  }));
}

/** "31%", "1,850 of 5,960 px", "stop 2, Services". */
export function scrollLine(
  at: number,
  stops: readonly BridgeStop[],
  scroll: { top: number; max: number } | null
): { percent: string; detail: string } {
  const fraction = scroll && scroll.max > 0 ? scroll.top / scroll.max : share(at, stops.length);
  const percent = `${Math.round(Math.min(1, Math.max(0, fraction)) * 100)}%`;
  const parts: string[] = [];
  if (scroll) parts.push(`${formatCount(scroll.top)} of ${formatCount(scroll.max)} px`);
  const stop = nearestStop(at, stops.length);
  const name = stops[stop]?.name;
  if (name) parts.push(`stop ${stop + 1}, ${name}`);
  return { percent, detail: parts.join(' · ') };
}

/** The next stop after `at` (or before it, `dir` -1), clamped. */
export function stepStop(at: number, dir: 1 | -1, count: number): number {
  if (count <= 0) return 0;
  const next = dir > 0 ? Math.floor(at + 1e-3) + 1 : Math.ceil(at - 1e-3) - 1;
  return Math.min(count - 1, Math.max(0, next));
}

// The server.

export type FrameState =
  | { kind: 'no-site' }
  | { kind: 'no-command'; message: string }
  | { kind: 'no-concept' }
  | { kind: 'busy'; project: string }
  | { kind: 'idle' }
  | { kind: 'starting'; status: ServerStatus }
  | { kind: 'failed'; status: ServerStatus }
  | { kind: 'running'; url: string; status: ServerStatus | null };

/** What the frame shows for the page view's state and source. */
export function frameState(view: PageView, conceptUrl: string | null): FrameState {
  if (view.settings.source === 'concept') {
    return conceptUrl ? { kind: 'running', url: conceptUrl, status: null } : { kind: 'no-concept' };
  }
  if (!view.siteRepo) return { kind: 'no-site' };
  const s = view.server;
  if (s.state === 'running' && s.url) return { kind: 'running', url: s.url, status: s };
  if (s.state === 'starting') return { kind: 'starting', status: s };
  if (s.state === 'failed') return { kind: 'failed', status: s };
  if (view.commandError) return { kind: 'no-command', message: view.commandError };
  if (view.busyWith) return { kind: 'busy', project: view.busyWith };
  return { kind: 'idle' };
}

/** Whether opening the view should start the server: only when nothing else uses it. */
export function shouldAutoStart(view: PageView): boolean {
  return (
    view.settings.source === 'site' &&
    Boolean(view.siteRepo) &&
    !view.commandError &&
    view.server.state === 'idle' &&
    !view.busyWith
  );
}

/** "localhost:4321 · running", for the source list and the status bar. */
export function serverLine(status: ServerStatus): string {
  const host = status.url ? shortAddress(status.url).replace(/\/$/, '') : null;
  switch (status.state) {
    case 'running':
      return `${host ?? 'dev server'} · ${status.external ? 'already running' : 'running'}`;
    case 'starting':
      return 'starting';
    case 'failed':
      return 'did not start';
    default:
      return 'stopped';
  }
}

/** What a frame loads for the view's source, and the address bar's text (the dev server's own). */
export function frameSource(
  view: PageView | null,
  options: { poster?: boolean } = {}
): { src: string | null; address: string } {
  if (!view) return { src: null, address: '' };
  if (view.settings.source === 'concept') {
    const concept = view.settings.concept;
    const name = concept ? (concept.split(/[\\/]/).pop() ?? concept) : 'No page concept';
    return { src: fileUrl(concept), address: name };
  }
  const running = view.server.state === 'running' ? view.server.url : null;
  // The frame loads the frame proxy in front of the server (it drops the
  // headers that forbid framing); the address bar names the server itself.
  const framed = running ? (view.server.frameUrl ?? running) : null;
  const address = pageAddress(framed, view.settings.path, options);
  const shown = pageAddress(running, view.settings.path);
  return {
    src: frameUrl(address),
    address: shown ? shortAddress(shown) : `${siteName(view.siteRepo)} dev server`,
  };
}
