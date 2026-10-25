/**
 * Renders the posters the page shows before (or instead of) the live 3D:
 * one frame per camera stop at 1920 x 912 (about the stage's shape on
 * common desktop windows, under the stop timeline) plus a 960 copy and a 16:9 thumbnail, and the portrait phone
 * poster. It opens the built site in headless Chromium with ?capture, which
 * hides everything but the canvas, and encodes WebP in the page itself. It
 * renders pages to files and never captures the screen.
 *
 *   pnpm --filter @maquette/site build
 *   PLAYWRIGHT=/path/to/playwright/index.mjs pnpm --filter @maquette/site posters
 *
 * PLAYWRIGHT defaults to the `playwright` package, which this repo does not
 * install; point it at any local copy. Run it again after the manifest or
 * the models change.
 */
import { writeFile } from 'node:fs/promises';
import { isAbsolute, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '..');
const out = resolve(root, 'public/scene/posters');
const importable = spec => (isAbsolute(spec) ? pathToFileURL(spec).href : spec);
const { chromium } = await import(importable(process.env.PLAYWRIGHT ?? 'playwright'));
const { preview } = await import('astro');

const STOPS = 7;
const WIDE = { width: 1920, height: 912 };
const PHONE = { width: 1024, height: 1268, out: [780, 966] };

const server = await preview({ root, logLevel: 'error' });
const base = `http://localhost:${server.port}/`;
const browser = await chromium.launch({
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'],
});

/** Encodes a PNG screenshot to WebP at the given sizes, inside the page. */
async function encode(page, png, sizes) {
  return page.evaluate(
    async ({ data, sizes }) => {
      const image = new Image();
      image.src = `data:image/png;base64,${data}`;
      await image.decode();
      return sizes.map(({ width, height, quality }) => {
        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d');
        // Cover crop, centred.
        const scale = Math.max(width / image.width, height / image.height);
        const w = width / scale;
        const h = height / scale;
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(
          image,
          (image.width - w) / 2,
          (image.height - h) / 2,
          w,
          h,
          0,
          0,
          width,
          height
        );
        return canvas.toDataURL('image/webp', quality).split(',')[1];
      });
    },
    { data: png.toString('base64'), sizes }
  );
}

async function open(viewport, query) {
  const context = await browser.newContext({ viewport });
  const page = await context.newPage();
  page.on('pageerror', error => console.error('page error:', error.message));
  await page.goto(`${base}?capture=${query}`);
  await page.waitForSelector('.tour.is-live', { timeout: 180_000 });
  return page;
}

async function settle(page) {
  // Capture cuts the camera to the stop; give the demand loop a few frames to draw.
  await page.evaluate(
    () => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)))
  );
  await page.waitForTimeout(600);
}

const saved = [];
async function save(name, base64) {
  const path = resolve(out, name);
  await writeFile(path, Buffer.from(base64, 'base64'));
  saved.push(path);
}

try {
  const page = await open(WIDE, 'stops');
  for (let stop = 0; stop < STOPS; stop++) {
    await page.evaluate(stop => {
      // A stop's framing is reached as its section's top meets the window's.
      const section = document.querySelectorAll('[data-stop-section]')[stop];
      window.scrollTo(0, section.getBoundingClientRect().top + window.scrollY);
    }, stop);
    await settle(page);
    const png = await page.locator('[data-canvas] canvas').screenshot({ type: 'png' });
    const [full, half, thumb] = await encode(page, png, [
      { ...WIDE, quality: 0.8 },
      { width: 960, height: 456, quality: 0.8 },
      { width: 192, height: 108, quality: 0.82 },
    ]);
    await save(`stop-${stop}-1920.webp`, full);
    await save(`stop-${stop}-960.webp`, half);
    await save(`stop-${stop}-thumb.webp`, thumb);
  }
  await page.context().close();

  const phone = await open({ width: PHONE.width, height: PHONE.height }, 'mobile');
  await settle(phone);
  const png = await phone.locator('[data-canvas] canvas').screenshot({ type: 'png' });
  const [poster] = await encode(phone, png, [
    { width: PHONE.out[0], height: PHONE.out[1], quality: 0.8 },
  ]);
  await save('hero-mobile.webp', poster);
} finally {
  await browser.close();
  await server.stop();
}

console.log(saved.join('\n'));
