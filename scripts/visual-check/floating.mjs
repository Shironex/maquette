/**
 * The overlap check the visual sets share: no floating approval card may
 * cover a button a view or a sheet keeps in its bottom bar.
 */

/** Fails the run when a floating card covers `button`. */
export async function assertClear(page, button, what) {
  const target = await button.boundingBox();
  const cards = await page.getByRole('alertdialog').all();
  for (const card of cards) {
    const box = await card.boundingBox();
    if (!target || !box) continue;
    const apart =
      box.x >= target.x + target.width ||
      target.x >= box.x + box.width ||
      box.y >= target.y + target.height ||
      target.y >= box.y + box.height;
    if (!apart) throw new Error(`A floating approval card covers ${what}`);
  }
}
