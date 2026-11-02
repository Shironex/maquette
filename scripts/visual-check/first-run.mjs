/**
 * The first run's shots for scripts/visual-check.mjs (ONLY=firstrun), each at
 * the normal size and at the 1120 by 720 minimum: the Board's Next step strip
 * (models in the Inbox, refs to send, a failed run, the repo with the longest
 * sentence, a cancelled run, Blender missing, failing or at a gone saved path), the Blender missing
 * screen in onboarding and those three in Settings, reached from the strip's button, and
 * the first run card on an empty website and game project. A website project
 * opens empty by switching to one of `?projects=many`; the Inbox's models then
 * keep the strip over the card, the least room the card ever gets, and
 * `&inbox=clear` shows it alone. Last, an image dropped on the Concept column,
 * on the Board and in the Lab right after.
 */

const HD = { width: 1120, height: 720 };
const SIZES = [
  { viewport: undefined, suffix: '' },
  { viewport: HD, suffix: '-1120' },
];

async function stripShots({ open, shot }, { viewport, suffix }) {
  for (const [query, button, name] of [
    ['&scenario=configured', 'Open Inbox', 'inbox'],
    ['&scenario=configured&inbox=clear', 'Send to 3D', 'send'],
    ['&scenario=configured&inbox=clear&pipeline=models', 'Open Pipeline', 'failed-run'],
    ['&scenario=idle&inbox=clear&pipeline=no-repo', 'Open Settings', 'repo'],
    ['&scenario=idle&inbox=clear&pipeline=cancelled', 'Open Pipeline', 'cancelled'],
  ]) {
    const board = await open(query, '#/board', viewport);
    const strip = board.getByRole('region', { name: 'Next step' });
    await strip.getByRole('button', { name: button }).waitFor();
    await board.mouse.move(0, 0);
    await shot(board, `first-run-strip-${name}${suffix}`);
    await board.context().close();
  }
}

/** Each Blender trouble in a configured workspace: the strip, then where its button leads. */
async function settingsShots({ open, shot }, { viewport, suffix }) {
  for (const [query, title, name] of [
    ['&scenario=configured&render=no-blender', 'Blender is missing', 'blender'],
    ['&scenario=configured&render=no-blender-run', 'Blender did not run', 'blender-run'],
    [
      '&scenario=idle&inbox=clear&pipeline=models&render=no-blender-path',
      'Your saved Blender path is gone',
      'blender-path',
    ],
  ]) {
    const board = await open(query, '#/board', viewport);
    const go = board
      .getByRole('region', { name: 'Next step' })
      .getByRole('button', { name: 'Open Toolchain' });
    await go.waitFor();
    await board.mouse.move(0, 0);
    await shot(board, `first-run-strip-${name}${suffix}`);
    await go.click();
    const screen = board.getByRole('region', { name: title });
    await screen.waitFor();
    await screen.scrollIntoViewIfNeeded();
    await board.mouse.move(0, 0);
    await shot(board, `first-run-settings-${name}${suffix}`);
    await board.context().close();
  }
}

async function onboardingShots({ open, shot }, { viewport, suffix }) {
  const page = await open('&scenario=no-blender', '', viewport);
  await page.getByRole('button', { name: 'Set up' }).click();
  await page.getByText('Empty folder, ready to use').waitFor();
  await page.getByRole('button', { name: 'Continue' }).click();
  await page.getByRole('region', { name: 'Blender is missing' }).waitFor();
  await page.mouse.move(0, 0);
  await shot(page, `first-run-onboarding-blender${suffix}`);
  // The screen pushes the tool list down, past the fold at the small size.
  await page.getByText('Node', { exact: true }).scrollIntoViewIfNeeded();
  await shot(page, `first-run-onboarding-blender-end${suffix}`);
  await page.context().close();
}

/** The card sits over the Board's columns: it must stay inside them, its steps in view. */
async function firstRunCard(page, shot, name) {
  const card = page.getByRole('region', { name: 'Your first build in three steps' });
  await card.waitFor();
  await page.mouse.move(0, 0);
  const { over, hidden } = await card.evaluate(el => {
    const box = el.getBoundingClientRect();
    const room = el.parentElement.getBoundingClientRect();
    const list = el.querySelector('ol');
    return {
      over: Math.max(room.top - box.top, box.bottom - room.bottom),
      hidden: list.scrollHeight - list.clientHeight,
    };
  });
  if (over > 0)
    console.error(`${name}: the first run card runs ${Math.ceil(over)} px past the Board`);
  if (hidden > 0) console.error(`${name}: the first run card scrolls ${hidden} px of its steps`);
  await shot(page, name);
  await page.context().close();
}

async function cardShots({ open, shot }, { viewport, suffix }) {
  for (const [query, name] of [
    ['&scenario=configured&projects=many', 'first-run-card'],
    ['&scenario=configured&projects=many&inbox=clear', 'first-run-card-no-strip'],
  ]) {
    const page = await open(query, '#/board', viewport);
    await page.locator('[aria-haspopup=menu]').first().click();
    await page.getByRole('menuitemradio', { name: /Starlit observatory/ }).click();
    await firstRunCard(page, shot, `${name}${suffix}`);
  }
  const game = await open('&scenario=configured&game=import', '#/board', viewport);
  await firstRunCard(game, shot, `first-run-card-game${suffix}`);
}

/** A file dropped on the empty Concept column: the card, then the Lab right after, not reloaded. */
async function dropShots({ open, shot }, { viewport, suffix }) {
  const page = await open('&scenario=configured&projects=many&inbox=clear', '#/board', viewport);
  await page.locator('[aria-haspopup=menu]').first().click();
  await page.getByRole('menuitemradio', { name: /Starlit observatory/ }).click();
  await page.getByRole('button', { name: 'Hide tips' }).click();
  await page.getByRole('button', { name: /Drop concept art/ }).waitFor();
  const files = await page.evaluateHandle(async () => {
    const canvas = Object.assign(document.createElement('canvas'), { width: 640, height: 480 });
    const paint = canvas.getContext('2d');
    paint.fillStyle = '#39406b';
    paint.fillRect(0, 0, 640, 480);
    paint.fillStyle = '#c9b8ff';
    paint.fillRect(260, 120, 120, 280);
    const blob = await new Promise(done => canvas.toBlob(done));
    const data = new DataTransfer();
    data.items.add(new File([blob], 'moon_gate.png', { type: 'image/png' }));
    return data;
  });
  const column = page.getByRole('region', { name: /^Concept/ });
  await column.dispatchEvent('drop', { dataTransfer: files });
  await column.getByText('Moon gate').waitFor();
  await page.waitForTimeout(300);
  await page.mouse.move(0, 0);
  await shot(page, `first-run-drop-board${suffix}`);
  await page.getByRole('button', { name: 'Open Lab' }).click();
  await page
    .getByRole('button', { name: /Moon gate/ })
    .first()
    .waitFor();
  await page.waitForTimeout(300);
  await shot(page, `first-run-drop-lab${suffix}`);
  await page.context().close();
}

export async function firstRunShots({ open, shot }) {
  for (const size of SIZES) {
    await dropShots({ open, shot }, size);
    await stripShots({ open, shot }, size);
    await settingsShots({ open, shot }, size);
    await onboardingShots({ open, shot }, size);
    await cardShots({ open, shot }, size);
  }
}
