/**
 * Settings > Agents' tool groups, for scripts/visual-check.mjs (ONLY=groups):
 * the list with every group on at 1440x900 and 1120x720, and with the
 * avatar and rig group turned off (the always on groups stay locked). The
 * off shot also checks a switch click leaves the page where it was.
 */

const SIZES = [
  { width: 1440, height: 900, name: '' },
  { width: 1120, height: 720, name: '-small' },
];

export async function toolGroupsShots({ open, shot }) {
  for (const size of SIZES) {
    const page = await open('&scenario=configured&agents=active', '#/settings', size);
    await page
      .getByRole('navigation', { name: 'Settings sections' })
      .getByRole('button', { name: 'Agents', exact: true })
      .click();
    const groups = page.getByRole('region', { name: 'Tool groups' });
    const avatar = groups.getByRole('switch', { name: 'Avatar and rig tools' });
    await avatar.waitFor();
    await groups.scrollIntoViewIfNeeded();
    await shot(page, `settings-tool-groups${size.name}`);
    await shot(groups, `settings-tool-groups-list${size.name}`);
    // The switch's own label, as a person clicks it.
    await avatar.locator('xpath=..').click();
    await groups.getByText('144 of 151 tools offered').waitFor();
    if (await avatar.isChecked()) throw new Error('the avatar and rig switch stayed on');
    await shot(page, `settings-tool-groups-off${size.name}`);
    await page.context().close();
  }
}
