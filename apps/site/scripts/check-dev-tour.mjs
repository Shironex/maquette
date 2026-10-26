/**
 * Run by hand, not in the gate: starts `astro dev` in this process, loads the
 * page in headless Chromium and checks that the 3D tour mounts in dev (no
 * React preamble error, `is-live` set), and stays live while scrolling.
 * Screenshots go to SHOTS when it is set. It never captures the screen.
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

const problems = [];
const check = (ok, what) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
  if (!ok) problems.push(what);
};

try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  page.on('console', m => m.type() === 'error' && errors.push(m.text()));
  page.on('pageerror', e => errors.push(String(e)));
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
  if (errors.length) console.log('console errors:\n' + errors.join('\n'));
} finally {
  await browser.close();
  await server.stop();
}
process.exit(problems.length ? 1 : 0);
