/**
 * The Maquette page bridge: when Maquette's Page view shows this page in a
 * frame, it drives the tour from its stop scrubber and reads back where the
 * page is and how fast it draws. The messages are the site contract the
 * root README lists under Page view; outside a frame the bridge stays off.
 * No three.js imports.
 */

/** Every message either way carries `maquette: BRIDGE_VERSION`. */
export const BRIDGE_VERSION = 1;

/** Frames kept for the frame time numbers, about two seconds at 60 fps. */
const FRAMES = 120;

/** How often the page sends its frame times, in milliseconds. */
const STATS_EVERY = 1000;

export interface BridgeStop {
  name: string;
}

/** What Maquette sends: `hello` asks for `ready`, `goto` scrolls to a stop. */
export type FromMaquette =
  { maquette: 1; type: 'hello' } | { maquette: 1; type: 'goto'; stop: number; smooth?: boolean };

export type ToMaquette =
  | { maquette: 1; type: 'ready'; stops: BridgeStop[]; features: string[] }
  | { maquette: 1; type: 'at'; stop: number; scroll: { top: number; max: number } }
  | { maquette: 1; type: 'stats'; frameMs: number; p95Ms: number };

/** Reads a message from Maquette, or null for anything else on the channel. */
export function readMessage(data: unknown): FromMaquette | null {
  if (typeof data !== 'object' || data === null) return null;
  const message = data as Record<string, unknown>;
  if (message.maquette !== BRIDGE_VERSION) return null;
  if (message.type === 'hello') return { maquette: 1, type: 'hello' };
  if (message.type === 'goto' && typeof message.stop === 'number' && Number.isFinite(message.stop))
    return { maquette: 1, type: 'goto', stop: message.stop, smooth: message.smooth === true };
  return null;
}

/** The mean and the 95th percentile of frame intervals, rounded to 0.1 ms. */
export function frameStats(deltas: readonly number[]): { frameMs: number; p95Ms: number } | null {
  if (deltas.length === 0) return null;
  const sorted = [...deltas].sort((a, b) => a - b);
  const mean = sorted.reduce((sum, d) => sum + d, 0) / sorted.length;
  const p95 = sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))] ?? mean;
  const round = (ms: number) => Math.round(ms * 10) / 10;
  return { frameMs: round(mean), p95Ms: round(p95) };
}

export interface BridgeOptions {
  stops: readonly BridgeStop[];
  /** The tour's progress, in stops, and a way to hear it change. */
  read: () => number;
  subscribe: (listener: (progress: number) => void) => () => void;
  /** Scrolls the page to fractional progress `stop`. */
  goto: (stop: number, smooth: boolean) => void;
}

/** Starts answering Maquette when the page runs inside its frame. */
export function startBridge(options: BridgeOptions): void {
  if (window.parent === window) return;
  const parent = window.parent;
  // Answers go back to whoever said hello; until then, nowhere.
  let origin: string | null = null;
  const send = (message: ToMaquette) => {
    if (origin) parent.postMessage(message, origin);
  };
  const at = (progress: number) => {
    const max = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
    send({
      maquette: 1,
      type: 'at',
      stop: Math.round(progress * 1000) / 1000,
      scroll: { top: Math.round(window.scrollY), max: Math.round(max) },
    });
  };

  let measuring = false;
  const measure = () => {
    if (measuring) return;
    measuring = true;
    const deltas: number[] = [];
    let last = performance.now();
    let sent = last;
    const frame = (now: number) => {
      deltas.push(now - last);
      if (deltas.length > FRAMES) deltas.shift();
      last = now;
      const stats = now - sent >= STATS_EVERY ? frameStats(deltas) : null;
      if (stats) {
        sent = now;
        send({ maquette: 1, type: 'stats', ...stats });
      }
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
  };

  let queued = false;
  options.subscribe(() => {
    if (!origin || queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      at(options.read());
    });
  });

  window.addEventListener('message', event => {
    if (event.source !== parent) return;
    const message = readMessage(event.data);
    if (!message) return;
    // An opaque origin (a file) can only be answered with '*'.
    origin = event.origin && event.origin !== 'null' ? event.origin : '*';
    if (message.type === 'hello') {
      send({
        maquette: 1,
        type: 'ready',
        stops: options.stops.map(s => ({ name: s.name })),
        features: ['goto', 'at', 'stats', 'poster'],
      });
      at(options.read());
      measure();
    } else {
      options.goto(message.stop, message.smooth === true);
    }
  });
}
