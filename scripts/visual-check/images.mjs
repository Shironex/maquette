/**
 * OpenAI's Image API for scripts/visual-check.mjs (ONLY=images): Settings >
 * Image generation with every kind on Codex and no key, with a saved key,
 * concepts on OpenAI Images and the month's spend, a key test (which only
 * lists the models), the Lab composer's cost line for a round, the same
 * round held at the cap, the queue naming OpenAI Images, an agent's round
 * waiting for approval (the prompt and its jobs in the queue) and an image
 * OpenAI refused. Mock mode's `?images=`, `?agents=images` and
 * `?lab=approval` states serve them.
 */

async function imagesSection(page) {
  const section = page.locator('#settings-images');
  await page
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('button', { name: 'Image generation' })
    .click();
  await section.waitFor();
  await section.scrollIntoViewIfNeeded();
  return section;
}

async function draftRound(page) {
  await page.getByLabel('Direction name').fill('Harbour');
  await page.getByLabel('Prompt').fill('A small harbour at dawn');
}

export async function imagesShots({ open, shot }) {
  const lab = '&scenario=configured&conceptMs=60000';

  const keychain = await open('&scenario=configured&images=on&promptMs=60000', '#/settings');
  const keychainSection = await imagesSection(keychain);
  await keychainSection.getByText(/^Waiting for the /).waitFor();
  await shot(keychainSection, 'images-settings-waiting-for-keychain');
  await keychain.context().close();

  const off = await open('&scenario=configured', '#/settings');
  const offSection = await imagesSection(off);
  await offSection.getByText('No key yet').waitFor();
  await shot(offSection, 'images-settings-off');
  await off.context().close();

  const on = await open('&scenario=configured&images=on', '#/settings');
  const onSection = await imagesSection(on);
  await onSection.getByText('$3.42').waitFor();
  await onSection.getByRole('button', { name: 'Test key' }).click();
  await onSection.getByText(/The key works and can use/).waitFor();
  await shot(onSection, 'images-settings-on');
  await shot(on, 'images-settings-page');
  await on.context().close();

  const round = await open(`${lab}&images=on`, '#/lab');
  await draftRound(round);
  const line = round.getByText(/^Runs on OpenAI Images \(low\)/);
  await line.waitFor();
  await line.scrollIntoViewIfNeeded();
  await shot(round, 'images-lab-round');
  await round.getByRole('button', { name: 'Generate 4 concepts' }).click();
  await round.getByText('Harbour · OpenAI Images').waitFor();
  await shot(round, 'images-lab-queue');
  await round.context().close();

  const cap = await open(`${lab}&images=cap`, '#/lab');
  await draftRound(cap);
  const held = cap.getByText(/would pass the monthly cap/);
  await held.waitFor();
  await held.scrollIntoViewIfNeeded();
  await shot(cap, 'images-lab-cap');
  await cap.context().close();

  const asked = await open('&scenario=configured&images=on&agents=images', '#/board');
  await asked
    .getByRole('alertdialog', { name: 'Make 2 images for Frozen lake arena with OpenAI Images?' })
    .waitFor();
  await shot(asked, 'images-agent-approval');
  await asked.context().close();

  const asking = await open(`${lab}&images=on&lab=approval`, '#/lab');
  const waiting = asking.getByText(/^Needs your approval/).first();
  await waiting.waitFor();
  await waiting.scrollIntoViewIfNeeded();
  await shot(asking, 'images-lab-approval');
  const queue = asking.getByRole('complementary', { name: 'Image queue' });
  const refused = asking.getByText(/^Refused for safety/).first();
  await refused.scrollIntoViewIfNeeded();
  await shot(queue, 'images-lab-refused');
  await asking.context().close();
}
