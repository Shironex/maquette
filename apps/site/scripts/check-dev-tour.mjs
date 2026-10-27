/**
 * Run by hand, not in the gate: starts `astro dev` in this process, loads the
 * page in headless Chromium and checks that the 3D tour mounts in dev (no
 * React preamble error, `is-live` set), stays live while scrolling with a
 * clean console (see ALLOWED), and that a software renderer gets the poster
 * instead. Screenshots go to SHOTS when it is set. It never captures the
 * screen.
 *
 *   PLAYWRIGHT=/path/to/playwright/index.mjs [SHOTS=dir] pnpm --filter @maquette/site check:dev-tour
 */
import { mkdir } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const importable = spec => (isAbsolute(spec) ? pathToFileURL(spec).href : spec);
const { chromium } = await import(importable(process.env.PLAYWRIGHT ?? 'playwright'));
const { dev } = await import('astro');

const shots = process.env.SHOTS ? resolve(process.env.SHOTS) : null;
if (shots) await mkdir(shots, { recursive: true });

// Port 0 asks for a free one, so a running dev server is left alone.
const server = await dev({ root, logLevel: 'error', server: { port: 0 } });
const base = `http://localhost:${server.address.port}/`;
const browser = await chromium.launch({
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'],
});

/**
 * Console warnings the check lets through, each for a cause outside this
 * code. R3F 9.8 makes a THREE.Clock for `state.clock`, and three r183+
 * deprecates Clock; the scene keeps reading `state.clock`. Check again on
 * each R3F bump. The second is Chromium's own driver note for the
 * SwiftShader GL this check runs on, not a page message.
 */
const ALLOWED = [
  /^THREE\.Clock: This module has been deprecated/,
  /GL Driver Message \(OpenGL, Performance, \w+, High\): GPU stall due to ReadPixels/,
];

const problems = [];
const check = (ok, what) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
  if (!ok) problems.push(what);
};

try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  const noise = [];
  page.on('console', m => {
    if (m.type() === 'error') errors.push(m.text());
    if ((m.type() === 'error' || m.type() === 'warning') && !ALLOWED.some(a => a.test(m.text())))
      noise.push(`${m.type()}: ${m.text()}`);
  });
  page.on('pageerror', e => {
    errors.push(String(e));
    noise.push(`pageerror: ${e}`);
  });
  await page.goto(base, { waitUntil: 'load' });

  const live = () => page.locator('.is-live').count();
  await page
    .waitForFunction(() => document.querySelector('.is-live'), null, { timeout: 90_000 })
    .catch(() => {});
  check((await live()) > 0, 'tour is live (is-live set)');
  check(!errors.some(e => /preamble/i.test(e)), 'no preamble error in the console');
  check((await page.locator('canvas').count()) > 0, 'a canvas exists');
  if (shots) await page.screenshot({ path: `${shots}/stop-0.png`, timeout: 180_000 });

  const height = await page.evaluate(() => document.documentElement.scrollHeight - innerHeight);
  for (const [i, f] of [
    [1, 0.2],
    [2, 0.35],
  ]) {
    await page.evaluate(y => scrollTo(0, y), height * f);
    await page.waitForTimeout(2500);
    check((await live()) > 0, `still live after scrolling to ${f}`);
    if (shots && i === 2) await page.screenshot({ path: `${shots}/stop-2.png`, timeout: 180_000 });
  }
  check(!errors.some(e => /preamble/i.test(e)), 'still no preamble error after scrolling');
  check(noise.length === 0, 'no console errors or warnings past the allowed ones');
  if (noise.length) console.log('console:\n' + noise.join('\n'));
} finally {
  await browser.close();
}

// Chromium's own software GL reports a major performance caveat, so the
// island must stay off and the page keep its poster.
const software = await chromium.launch({ args: ['--use-gl=swiftshader'] });
try {
  const page = await software.newPage({ viewport: { width: 1600, height: 900 } });
  await page.goto(base, { waitUntil: 'load' });
  const caveat = await page.evaluate(
    () =>
      !document.createElement('canvas').getContext('webgl2', { failIfMajorPerformanceCaveat: true })
  );
  if (!caveat) {
    console.log('skip software GL: this Chromium reports no caveat for it');
  } else {
    // The island starts after idle time (at most 1.5 s); give it longer than that.
    await page.waitForTimeout(4000);
    check((await page.locator('.is-live').count()) === 0, 'software GL: tour stays on posters');
    check((await page.locator('canvas').count()) === 0, 'software GL: no canvas');
    const poster = await page.evaluate(() => {
      const img = document.querySelector('[data-poster].is-shown');
      return img instanceof HTMLImageElement && img.complete && img.naturalWidth > 0;
    });
    check(poster, 'software GL: the stop poster is shown');
    if (shots) await page.screenshot({ path: `${shots}/software-gl.png` });
  }
} finally {
  await software.close();
  await server.stop();
}
process.exit(problems.length ? 1 : 0);
