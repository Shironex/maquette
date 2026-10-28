/**
 * Runs axe over mock mode in headless Chromium: every view and its main
 * dialogs and sheets (scripts/a11y-check/cases.mjs), against WCAG 2.2 A and
 * AA and axe's best practices. It prints every violation and exits 1 when one
 * is serious or critical. It reads pages only and never captures the screen.
 *
 *   pnpm a11y
 *
 * It starts its own dev server on a free port. PORT reuses the dev server on
 * that port instead (or starts one there), ONLY=<text> runs the cases whose
 * name holds that text, ALL=1 fails on moderate and minor violations too and
 * PLAYWRIGHT picks another local Playwright `index.mjs`.
 */
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { createServer as createNetServer } from 'node:net';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { cases } from './a11y-check/cases.mjs';

const root = resolve(import.meta.dirname, '..');
const FAILING = process.env.ALL
  ? ['critical', 'serious', 'moderate', 'minor']
  : ['critical', 'serious'];
const TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa', 'best-practice'];

const importable = spec => (isAbsolute(spec) ? pathToFileURL(spec).href : spec);
const { chromium } = await import(importable(process.env.PLAYWRIGHT ?? 'playwright'));
const axeSource = await readFile(
  createRequire(import.meta.url).resolve('axe-core/axe.min.js'),
  'utf8'
);

/** A port nothing listens on, so another worktree's dev server is never the one checked. */
function freePort() {
  return new Promise((done, fail) => {
    const probe = createNetServer().once('error', fail);
    probe.listen(0, '127.0.0.1', () => {
      const { port } = probe.address();
      probe.close(() => done(port));
    });
  });
}

const port = process.env.PORT ? Number(process.env.PORT) : await freePort();
const base = `http://localhost:${port}/`;

async function serverUp() {
  if (!process.env.PORT) return false;
  try {
    return (await fetch(base)).ok;
  } catch {
    return false;
  }
}

/** Reuses the dev server on PORT, else starts one in this process. */
async function startVite() {
  if (await serverUp()) return null;
  const { createServer } = await import(
    importable(resolve(root, 'apps/web/node_modules/vite/dist/node/index.js'))
  );
  const server = await createServer({
    root: resolve(root, 'apps/web'),
    configFile: resolve(root, 'apps/web/vite.config.ts'),
    server: { port, strictPort: true },
    logLevel: 'error',
  });
  await server.listen();
  return server;
}

/** Axe's violations on the page as it stands. */
async function audit(page) {
  await page.addScriptTag({ content: axeSource });
  return page.evaluate(
    tags =>
      window.axe.run(document, {
        runOnly: { type: 'tag', values: tags },
        resultTypes: ['violations'],
      }),
    TAGS
  );
}

const only = process.env.ONLY;
const chosen = cases.filter(c => !only || c.name.includes(only));
const vite = await startVite();
const browser = await chromium.launch();
let failures = 0;
let found = 0;

try {
  for (const c of chosen) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    context.setDefaultTimeout(30_000);
    const page = await context.newPage();
    try {
      await page.goto(`${base}?delay=50${c.query}${c.hash ?? ''}`);
      await c.ready(page);
      await page.waitForTimeout(400);
      const { violations } = await audit(page);
      for (const v of violations) {
        found += 1;
        if (FAILING.includes(v.impact)) failures += 1;
        console.log(`${c.name}: [${v.impact}] ${v.id}: ${v.help}`);
        for (const node of v.nodes.slice(0, 6))
          console.log(
            `    ${node.target.join(' ')}  ${node.failureSummary?.split('\n')[1]?.trim() ?? ''}`
          );
        if (v.nodes.length > 6) console.log(`    and ${v.nodes.length - 6} more`);
      }
      if (violations.length === 0) console.log(`${c.name}: clean`);
    } catch (error) {
      failures += 1;
      console.log(`${c.name}: could not reach the state to check: ${error.message.split('\n')[0]}`);
    } finally {
      await context.close();
    }
  }
} finally {
  await browser.close();
  await vite?.close();
}

console.log(
  `\n${chosen.length} cases, ${found} violations, ${failures} failing (${FAILING.join(', ')}).`
);
process.exitCode = failures > 0 ? 1 : 0;
