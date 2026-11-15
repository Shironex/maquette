/**
 * The overlap check the visual sets share: no approval card may cover a
 * button, a row or the numbers a view or a sheet shows. The cards dock in a
 * tray beside the view, and a sheet opens over that tray, so a card behind a
 * sheet's backdrop overlaps on paper but covers nothing.
 */

/** Fails the run when an approval card covers `target` on screen. */
export async function assertClear(page, target, what) {
  const box = await target.boundingBox();
  const cards = await page.getByRole('alertdialog').all();
  for (const card of cards) {
    const area = await card.boundingBox();
    if (!box || !area) continue;
    const left = Math.max(box.x, area.x);
    const right = Math.min(box.x + box.width, area.x + area.width);
    const top = Math.max(box.y, area.y);
    const bottom = Math.min(box.y + box.height, area.y + area.height);
    if (left >= right || top >= bottom) continue;
    const onTop = await card.evaluate(
      (el, [x, y]) => el.contains(document.elementFromPoint(x, y)),
      [(left + right) / 2, (top + bottom) / 2]
    );
    if (onTop) throw new Error(`An approval card covers ${what}`);
  }
}

/** Waits for the approval tray and opens it when a narrow window started it folded. */
export async function showCards(page) {
  const tray = page.getByRole('complementary', { name: 'Waiting for approval' });
  await tray.waitFor();
  const show = tray.getByRole('button', { name: 'Show the approval cards' });
  if (await show.count()) await show.click();
  await page.getByRole('alertdialog').first().waitFor();
}
