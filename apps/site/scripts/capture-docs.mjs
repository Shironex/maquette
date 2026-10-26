/**
 * Screenshots of the built docs in headless Chromium, at desktop and phone
 * width: the home, the first project guide, Compress, the MCP tools, the
 * search results and the phone menu drawer. It renders pages to files and
 * never captures the screen.
 *
 *   pnpm --filter @maquette/site build
 *   PLAYWRIGHT=/path/to/playwright/index.mjs OUT_DIR=/tmp/shots node scripts/capture-docs.mjs
 *
 * PLAYWRIGHT defaults to the `playwright` package, which this repo does not
 * install; point it at any local copy. OUT_DIR defaults to `.visual/`. PORT
 * picks the preview server port.
 */
import { mkdir } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const out = resolve(process.env.OUT_DIR ?? resolve(root, '../../.visual'));
const port = Number(process.env.PORT ?? 15306);
const importable = spec => (isAbsolute(spec) ? pathToFileURL(spec).href : spec);
const { chromium } = await import(importable(process.env.PLAYWRIGHT ?? 'playwright'));
const { preview } = await import('astro');

const SIZES = {
  desktop: { width: 1440, height: 900 },
  phone: { width: 390, height: 844 },
};
const PAGES = [
  ['home', '/docs/'],
  ['guide', '/docs/start/first-project/'],
  ['compress', '/docs/pipeline/compress/'],
  ['mcp-tools', '/docs/agents/mcp-tools/'],
];

await mkdir(out, { recursive: true });
const server = await preview({ root, logLevel: 'error', server: { port } });
const base = `http://localhost:${server.port}`;
const browser = await chromium.launch();

try {
  for (const [size, viewport] of Object.entries(SIZES)) {
    const context = await browser.newContext({ viewport, deviceScaleFactor: 1 });
    const page = await context.newPage();
    const shoot = (name, options = {}) =>
      page.screenshot({ path: resolve(out, `docs-${name}-${size}.png`), ...options });

    for (const [name, path] of PAGES) {
      await page.goto(base + path, { waitUntil: 'networkidle' });
      await shoot(name, { fullPage: true });
    }

    await page.goto(`${base}/docs/start/first-project/`, { waitUntil: 'networkidle' });
    await page.locator('site-search button[data-open-modal]').click();
    await page.getByPlaceholder('Search').fill('ktx2');
    await page.locator('.pagefind-ui__result').first().waitFor();
    await shoot('search');
    await page.keyboard.press('Escape');

    if (size === 'phone') {
      await page.goto(`${base}/docs/start/first-project/`, { waitUntil: 'networkidle' });
      await page.getByRole('button', { name: /menu/i }).first().click();
      await page.waitForTimeout(300);
      await shoot('drawer');
    }
    await context.close();
  }
} finally {
  await browser.close();
  await server.stop();
}
console.log(`docs shots in ${out}`);
