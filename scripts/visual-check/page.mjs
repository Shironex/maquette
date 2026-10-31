/**
 * The Page view's shots for scripts/visual-check.mjs (ONLY=page): the site
 * dev page with its hotspots, running at a stop, its header's server state and Stop, stopped
 * from there, starting, failed, a project without a site
 * repo, a phone, the Devices tab, a page concept, and the compare board
 * (ship against this run on desktop, then with the poster on a phone),
 * a tour of ten stops on the scrubber, the mock site with the bridge shim
 * instead of its own bridge, and a site that forbids framing and has no
 * bridge, shown through the real frame proxy and driven by its shim.
 * Except for that last one, the dev server and its page are mock mode's
 * own.
 */
import { checkHeaders, startForbiddingSite, startFrameProxy } from './framed-site.mjs';

const RUNNING = '&scenario=configured&page=running';

/** Waits until the page in the main frame has answered the bridge. */
async function answered(page) {
  await page.getByRole('button', { name: /^Jump to stop 2, / }).waitFor();
  await page.getByText(/^p95 /).first().waitFor();
}

export async function pageShots({ open, shot }) {
  const hot = await open(`${RUNNING}&timeline=looks`, '#/page');
  await answered(hot);
  await hot.getByRole('button', { name: 'Hotspots' }).click();
  await hot.getByRole('button', { name: /^Jump to stop 3, / }).click();
  await hot.waitForTimeout(1200);
  await shot(hot, 'page-hotspots');
  await hot.context().close();

  const running = await open(RUNNING, '#/page');
  await answered(running);
  await running.getByRole('button', { name: /^Jump to stop 2, / }).click();
  await running.getByText(/stop 2, Veil gate/).waitFor();
  await shot(running, 'page-running');

  await running
    .getByRole('group', { name: 'Device' })
    .getByRole('button', { name: 'Phone' })
    .click();
  await running.getByText('p95 38.2').waitFor();
  await shot(running, 'page-phone');

  await running
    .getByRole('group', { name: 'Device' })
    .getByRole('button', { name: 'Desktop' })
    .click();
  await running.getByRole('button', { name: 'Compare' }).click();
  await running.getByRole('region', { name: 'Current ship' }).waitFor();
  await running.getByText('Ships next').waitFor();
  await shot(running, 'page-ship-run');

  await running
    .getByRole('group', { name: 'Device' })
    .getByRole('button', { name: 'Phone' })
    .click();
  await running
    .getByRole('group', { name: 'Compare' })
    .getByRole('button', { name: 'With poster' })
    .click();
  await running.locator('section[aria-label="Poster fallback"]').waitFor();
  await running.getByText('Phone verdict').waitFor();
  await running.waitForTimeout(600);
  await shot(running, 'page-compare');

  await running
    .getByRole('group', { name: 'Compare' })
    .getByRole('button', { name: 'One' })
    .click();
  await running.getByRole('tab', { name: 'Devices' }).click();
  await running.getByRole('region', { name: 'Laptop' }).waitFor();
  await running.waitForTimeout(600);
  await shot(running, 'page-devices');

  await running.getByRole('tab', { name: 'Preview' }).click();
  await running.getByText('HTML concept', { exact: true }).click();
  await running.getByText('Mock', { exact: true }).waitFor();
  await running.waitForTimeout(400);
  await shot(running, 'page-concept');
  await running.context().close();

  // The header says where the server runs and who started it; Stop ends it.
  const stop = await open(RUNNING, '#/page');
  await answered(stop);
  const stopButton = stop.getByRole('button', { name: 'Stop', exact: true });
  await stopButton.hover();
  await stop.getByRole('tooltip').waitFor();
  await shot(stop, 'page-server-stop-tip');
  await stopButton.click();
  await stop.getByRole('heading', { name: 'The dev server is stopped' }).waitFor();
  await stop.mouse.move(0, 0);
  await shot(stop, 'page-server-stopped');
  await stop.context().close();

  const starting = await open('&scenario=configured&page=starting', '#/page');
  await starting.getByRole('heading', { name: "Starting the site's dev server" }).waitFor();
  await shot(starting, 'page-starting');
  await starting.context().close();

  const failed = await open('&scenario=configured&page=failed', '#/page');
  await failed.getByText("The site's dev server did not start").waitFor();
  await shot(failed, 'page-failed');
  await failed.context().close();

  const noSite = await open('&scenario=configured&page=no-site', '#/page');
  await noSite.getByRole('heading', { name: 'No site repo yet' }).waitFor();
  await shot(noSite, 'page-no-site');
  await noSite.context().close();

  await manyStopsShots({ open, shot });
  await shimShots({ open, shot });
  await framedShots({ open, shot });
}

const SHIM_HINT = "Maquette's shim scrolls the page: the site has no page bridge";

/** The page in the main frame, by the address it loaded. */
function pageFrame(page, prefix) {
  const frame = page.frames().find(f => f.url().startsWith(prefix));
  if (!frame) throw new Error(`no frame at ${prefix}`);
  return frame;
}

/** Mock mode's site without its bridge, carrying the shim: it answers as the shim. */
async function shimShots({ open, shot }) {
  const shimmed = await open('&scenario=configured&page=shim', '#/page');
  await answered(shimmed);
  await shimmed.getByText(SHIM_HINT).waitFor();
  await shimmed.getByText(/Measured by Maquette's shim/).waitFor();
  await shimmed.getByRole('button', { name: /^Jump to stop 4, / }).click();
  await shimmed.getByText(/ · stop 4, /).waitFor();
  await shimmed.waitForTimeout(800);
  await shot(shimmed, 'page-shim');
  await shimmed.context().close();
}

/** Ten stops with long names: every label keeps to its slot. */
async function manyStopsShots({ open, shot }) {
  const many = await open('&scenario=configured&page=stops-10', '#/page');
  await answered(many);
  await many.getByRole('button', { name: /^Jump to stop 10, Contact$/ }).waitFor();
  await many.getByRole('button', { name: /^Jump to stop 9, / }).click();
  await many.getByText(/stop 9, Testimonials/).waitFor();
  await shot(many, 'page-stops-10');
  await shot(many.getByRole('region', { name: 'Scroll scrubber' }), 'page-stops-10-scrubber');
  await many.context().close();
}

/**
 * A dev server that sends frame-ancestors 'none' and X-Frame-Options: DENY
 * and has no page bridge, behind the frame proxy: the page and its HMR
 * socket show, the shim fills the frame time, a stop click scrolls the
 * page, and scrolling the page moves the scrubber.
 */
async function framedShots({ open, shot }) {
  const site = await startForbiddingSite();
  let proxy = null;
  try {
    proxy = await startFrameProxy(site.url);
    await checkHeaders(site.url, proxy.url);
    const query = new URLSearchParams({ site: site.url, frame: proxy.url });
    const framed = await open(`&scenario=configured&page=framed&${query}`, '#/page');
    await answered(framed);
    await framed
      .frameLocator('iframe[title]')
      .first()
      .getByText(/^HMR socket: connected through 127\.0\.0\.1:/)
      .waitFor();
    await framed.getByText(site.url.replace('http://', '').replace(/\/$/, '')).first().waitFor();
    await framed.getByText(SHIM_HINT).waitFor();
    await framed.getByText(/Measured by Maquette's shim/).waitFor();
    const page = pageFrame(framed, proxy.url);
    // Five full height sections spread evenly: stop 3 is two screens down.
    await framed.getByRole('button', { name: /^Jump to stop 3, / }).click();
    await page.waitForFunction(() => Math.abs(scrollY - 2 * innerHeight) < 2);
    await framed.getByText(/ · stop 3, /).waitFor();
    await shot(framed, 'page-framed');
    await page.evaluate(() => scrollTo({ top: innerHeight * 3.25, behavior: 'instant' }));
    await framed.getByText(/^81% · /).waitFor();
    await framed.getByText(/ · stop 4, /).waitFor();
    await shot(framed, 'page-framed-scrolled');
    await framed.context().close();
  } finally {
    await proxy?.stop();
    await site.stop();
  }
}
