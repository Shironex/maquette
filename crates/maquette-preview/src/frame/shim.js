/**
 * Maquette's page bridge shim. The frame proxy serves this file at
 * /__maquette/bridge.js on its own origin and adds it to every page it
 * frames, so a site without the page bridge (the root README, Page view)
 * still follows the stop scrubber, without editing the site.
 *
 * It speaks bridge v1 the way a site would, with `source: 'shim'` on every
 * message: `goto` scrolls to the element whose id or `data-maquette-stop`
 * names the stop, else spreads the stops evenly over the scroll range;
 * `at` reports where a scroll left the page; `stats` sends frame times
 * from requestAnimationFrame. It answers only the parent window, and only
 * once the page has loaded. A page with its own bridge sets
 * `window.__maquetteBridge`, and the shim then stands down for good.
 *
 * It reads every browser global through `window`, so a test can run it
 * against a fake one.
 */
(window => {
  'use strict';
  if (window.parent === window || window.__maquetteShim) return;
  window.__maquetteShim = true;

  const document = window.document;
  const parent = window.parent;
  /** Frames kept for the frame time numbers, about two seconds at 60 fps. */
  const FRAMES = 120;
  /** How often the frame times go out, in milliseconds. */
  const STATS_EVERY = 1000;
  /** How long after load the shim waits, so a bridge of the page's own answers first. */
  const GRACE = 300;
  const MAX_STOPS = 50;

  let origin = null;
  let stops = [];
  let answering = false;
  let off = false;
  let frame = 0;
  let queued = false;

  const send = message => {
    if (origin && !off) parent.postMessage({ maquette: 1, source: 'shim', ...message }, origin);
  };
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  const round = (v, step) => Math.round(v / step) * step;
  const slug = name =>
    String(name)
      .normalize('NFKD')
      .replace(/[̀-ͯ]/g, '')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');

  const scroller = () => document.scrollingElement || document.documentElement;
  const maxScroll = () => Math.max(0, scroller().scrollHeight - window.innerHeight);

  /** The element that marks stop `i`: `data-maquette-stop` (its name, slug or number), else an id. */
  function anchor(name, i) {
    const want = slug(name);
    for (const el of document.querySelectorAll('[data-maquette-stop]')) {
      const value = (el.getAttribute('data-maquette-stop') || '').trim();
      if (value === name || value === String(i + 1) || (want && slug(value) === want)) return el;
    }
    return (want && document.getElementById(want)) || document.getElementById(name);
  }

  /** Where an element sits once scrolled into view, its scroll margin and the page's padding kept. */
  function topOf(el) {
    const style = window.getComputedStyle(el);
    const padding = parseFloat(window.getComputedStyle(scroller()).scrollPaddingTop) || 0;
    const margin = parseFloat(style.scrollMarginTop) || 0;
    return el.getBoundingClientRect().top + window.scrollY - margin - padding;
  }

  /** Each stop's scroll top, never going back up the page. */
  function tops() {
    const max = maxScroll();
    const n = stops.length;
    let last = 0;
    return stops.map((stop, i) => {
      const el = anchor(stop.name, i);
      const even = n > 1 ? (i / (n - 1)) * max : 0;
      last = Math.max(last, clamp(el ? topOf(el) : even, 0, max));
      return last;
    });
  }

  /** The scroll top for fractional stop `stop`: 2.5 is halfway from stop 3 to 4. */
  function scrollFor(stop, at) {
    if (at.length === 0) return 0;
    const s = clamp(stop, 0, at.length - 1);
    const i = Math.floor(s);
    const next = at[Math.min(at.length - 1, i + 1)];
    return at[i] + (s - i) * (next - at[i]);
  }

  /** The fractional stop at scroll top `top`. */
  function stopAt(top, at) {
    if (at.length < 2) return 0;
    if (top <= at[0]) return 0;
    for (let i = 0; i < at.length - 1; i++) {
      const span = at[i + 1] - at[i];
      if (top < at[i + 1]) return span > 0 ? i + (top - at[i]) / span : i;
    }
    return at.length - 1;
  }

  /** The mean and the 95th percentile of frame intervals, as the site bridge sends them. */
  function frameStats(deltas) {
    const sorted = [...deltas].sort((a, b) => a - b);
    const mean = sorted.reduce((sum, d) => sum + d, 0) / sorted.length;
    const p95 = sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))];
    return { frameMs: round(mean, 0.1), p95Ms: round(p95, 0.1) };
  }

  /** A page with its own bridge said so: no answer from here again. */
  const native = () => Boolean(window.__maquetteBridge);
  function standDown() {
    off = true;
    window.cancelAnimationFrame(frame);
    window.removeEventListener('message', onMessage);
    window.removeEventListener('scroll', onScroll);
    window.removeEventListener('resize', onScroll);
  }

  function at() {
    if (native()) return standDown();
    const top = window.scrollY;
    send({
      type: 'at',
      stop: round(stopAt(top, tops()), 0.001),
      scroll: { top: Math.round(top), max: Math.round(maxScroll()) },
    });
  }

  function onScroll() {
    if (!answering || queued) return;
    queued = true;
    window.requestAnimationFrame(() => {
      queued = false;
      at();
    });
  }

  function measure() {
    if (frame) return;
    const deltas = [];
    let last = window.performance.now();
    let sent = last;
    const step = now => {
      if (native()) return standDown();
      deltas.push(now - last);
      if (deltas.length > FRAMES) deltas.shift();
      last = now;
      if (now - sent >= STATS_EVERY) {
        sent = now;
        send({ type: 'stats', ...frameStats(deltas) });
      }
      frame = window.requestAnimationFrame(step);
    };
    frame = window.requestAnimationFrame(step);
  }

  function answer() {
    if (native()) return standDown();
    answering = true;
    send({ type: 'ready', stops, features: ['goto', 'at', 'stats'] });
    at();
    measure();
  }

  /** Runs `then` once the page has loaded and the grace has passed. */
  let settling = false;
  function settled(then) {
    if (settling) return;
    settling = true;
    const wait = () =>
      window.setTimeout(() => {
        settling = false;
        then();
      }, GRACE);
    if (document.readyState === 'complete') wait();
    else window.addEventListener('load', wait, { once: true });
  }

  function readStops(list) {
    if (!Array.isArray(list)) return null;
    return list
      .map(s => (s && typeof s.name === 'string' ? s.name.trim().slice(0, 60) : ''))
      .filter(Boolean)
      .slice(0, MAX_STOPS)
      .map(name => ({ name }));
  }

  function onMessage(event) {
    if (event.source !== parent) return;
    const m = event.data;
    if (!m || typeof m !== 'object' || m.maquette !== 1) return;
    if (native()) return standDown();
    // An opaque origin (a file) can only be answered with '*'.
    origin = event.origin && event.origin !== 'null' ? event.origin : '*';
    if (m.type === 'hello') {
      stops = readStops(m.stops) || stops;
      if (answering) answer();
      else settled(answer);
    } else if (m.type === 'goto' && answering && Number.isFinite(m.stop)) {
      const top = scrollFor(m.stop, tops());
      window.scrollTo({ top, behavior: m.smooth === true ? 'smooth' : 'instant' });
    }
  }

  window.addEventListener('message', onMessage);
  window.addEventListener('scroll', onScroll, { passive: true });
  window.addEventListener('resize', onScroll);
})(window);
