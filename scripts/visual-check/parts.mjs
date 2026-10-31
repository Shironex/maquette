/**
 * Parts for scripts/visual-check.mjs (ONLY=parts): the Inbox's "One mesh,
 * no parts" check with Split into parts added and its preview numbers, the
 * split preview coloured by part, the Inspector's Parts tab with two parts
 * selected and one hidden, a rename in place, before and after the split
 * side by side, and a model that is still one mesh, then the split and
 * assemble set (./split.mjs). Mock mode's `?parts=demo` frost lantern and
 * rune chest stand in for real models.
 */

import { splitShots } from './split.mjs';

const PARTS = '&scenario=configured&parts=demo';

export async function partsShots({ open, shot }) {
  const inbox = await open(PARTS, '#/inbox');
  await inbox.getByRole('heading', { name: 'Automatic checks' }).waitFor();
  await inbox
    .getByRole('button', { name: /^Frost lantern/ })
    .first()
    .click();
  await inbox.getByText('One mesh, no parts').waitFor();
  await inbox.getByRole('button', { name: 'Split into parts', pressed: false }).first().click();
  await inbox.getByRole('button', { name: 'Split into parts added' }).waitFor();
  await inbox.waitForTimeout(1500);
  await shot(inbox, 'parts-inbox-check');
  await inbox.getByRole('button', { name: 'Preview the numbers' }).click();
  await inbox.getByText(/^By loose pieces: /).waitFor();
  await shot(inbox, 'parts-inbox-preview');
  await inbox
    .getByRole('group', { name: 'Preview mode' })
    .getByRole('button', { name: 'Parts' })
    .click();
  await inbox.getByText('Split preview · by loose pieces · on a copy').waitFor();
  await inbox.waitForTimeout(1500);
  await shot(inbox, 'parts-inbox-coloured');
  await inbox.context().close();

  const tab = await open(PARTS, '#/inspector');
  await tab.getByRole('tab', { name: 'Parts' }).click();
  const list = tab.getByRole('list', { name: 'Parts' });
  await list.waitFor();
  await list.getByRole('button', { name: 'cage' }).click();
  await list.getByRole('button', { name: 'crystal' }).click({ modifiers: ['Shift'] });
  await tab.getByText('cage and crystal selected').waitFor();
  await tab.waitForTimeout(1500);
  await shot(tab, 'parts-tab');
  await tab.getByRole('button', { name: 'Tweak materials' }).click();
  await tab.getByRole('combobox', { name: 'Applies to' }).waitFor();
  await shot(tab, 'parts-tweak-materials');
  await tab.getByRole('button', { name: 'Tweak materials' }).click();
  await list.getByRole('button', { name: 'post' }).dblclick();
  await tab.getByRole('textbox', { name: 'Rename post' }).fill('pillar');
  await shot(tab.getByRole('complementary', { name: 'Parts' }), 'parts-rename');
  await tab.keyboard.press('Escape');

  await tab
    .getByRole('group', { name: 'Parts page' })
    .getByRole('button', { name: 'Before and after' })
    .click();
  await tab.getByRole('table', { name: 'Before and after' }).getByText('6 pass').waitFor();
  await tab.waitForTimeout(2000);
  await shot(tab, 'parts-compare');

  await tab
    .getByRole('group', { name: 'Parts page' })
    .getByRole('button', { name: 'Parts' })
    .click();
  await tab.getByRole('combobox', { name: 'Selected model' }).selectOption({ label: 'Rune chest' });
  await tab.getByRole('heading', { name: 'One mesh, no parts' }).waitFor();
  await tab.waitForTimeout(1200);
  await shot(tab, 'parts-one-mesh');
  await tab.context().close();

  await splitShots({ open, shot });
}
