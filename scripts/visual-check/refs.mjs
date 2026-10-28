/**
 * Multi-view refs for scripts/visual-check.mjs (ONLY=refs), on Frostveil
 * with the guardian statues' refs tagged (`?lab=refs`): the refs sheet
 * with 1 of 4 views and its Other tray, a view chip's menu, Make the other
 * views before and after its candidates come back, Clean background with
 * its before and after, and the weak ref warnings in Send to 3D and in the
 * batch send panel, with the batch approval repeating the count, and the
 * detail tagged as the back, which looks different from the front.
 */

export async function refsShots({ open, shot }) {
  const board = await open('&scenario=configured&lab=refs&conceptMs=1500', '#/board');
  await board.getByRole('button', { name: 'More for Guardian statues' }).click();
  await board.getByRole('menuitem', { name: 'Refs and views...' }).click();
  const sheet = board.getByRole('dialog', { name: 'Guardian statues refs' });
  await sheet.getByText('Kept with the asset, never sent as views').waitFor();
  await board.waitForTimeout(300);
  await shot(board, 'refs-sheet');
  await sheet.getByRole('button', { name: 'View of guardian_statues_crop.png: Other' }).click();
  await board.getByRole('menu', { name: 'View of guardian_statues_crop.png' }).waitFor();
  await shot(board, 'refs-sheet-chip-menu');
  await board.keyboard.press('Escape');

  await sheet.getByRole('button', { name: 'Make the other views' }).click();
  const views = board.getByRole('dialog', { name: 'Make the other views of Guardian statues' });
  await views.getByRole('button', { name: 'Draw 3 views' }).waitFor();
  await shot(board, 'refs-make-views');
  await views.getByRole('button', { name: 'Draw 3 views' }).click();
  await views.getByRole('button', { name: 'Use as Right' }).first().waitFor({ timeout: 20_000 });
  await board.waitForTimeout(400);
  await shot(board, 'refs-make-views-candidates');
  await views.getByRole('button', { name: 'Use as Left' }).first().click();
  await views.getByRole('button', { name: 'In use' }).waitFor();
  await views.getByRole('button', { name: 'Back to the refs' }).click();

  await sheet.getByRole('button', { name: 'View of guardian_statues_crop.png: Other' }).click();
  await board.getByRole('menuitem', { name: 'Clean background...' }).click();
  const clean = board.getByRole('dialog', { name: 'Clean background, Guardian statues' });
  await clean.getByRole('button', { name: 'Clean background' }).waitFor();
  await shot(board, 'refs-clean-before');
  await clean.getByRole('button', { name: 'Clean background' }).click();
  await clean.getByRole('button', { name: 'Use the clean copy' }).waitFor({ timeout: 20_000 });
  await board.waitForTimeout(400);
  await shot(board, 'refs-clean-after');
  await clean.getByRole('button', { name: 'Keep the original' }).click();
  await sheet.getByRole('button', { name: 'Done' }).click();
  await board.context().close();

  const send = await open('&scenario=configured&lab=refs', '#/board');
  await send.getByRole('button', { name: 'More for Guardian statues' }).click();
  await send.getByRole('menuitem', { name: 'Send to 3D' }).click();
  const panel = send.getByRole('dialog', { name: 'Send Guardian statues to 3D' });
  await panel
    .getByRole('group', { name: 'Provider' })
    .getByRole('button', { name: 'Tripo' })
    .click();
  await panel.getByText('Multi-view', { exact: true }).click();
  await panel.getByText('One view only: the sides and back are guessed').waitFor();
  await panel.getByText(/ref warnings?$/).waitFor();
  await send.waitForTimeout(300);
  await shot(send, 'refs-send-warnings');
  await send.context().close();

  const differs = await open('&scenario=configured&lab=refs', '#/board');
  await differs.getByRole('button', { name: 'More for Guardian statues' }).click();
  await differs.getByRole('menuitem', { name: 'Refs and views...' }).click();
  const tags = differs.getByRole('dialog', { name: 'Guardian statues refs' });
  await tags.getByRole('button', { name: 'View of guardian_statues_detail.png: Other' }).click();
  await differs
    .getByRole('menu', { name: 'View of guardian_statues_detail.png' })
    .getByRole('menuitem', { name: 'Back', exact: true })
    .click();
  await tags.getByRole('button', { name: 'View of guardian_statues_detail.png: Back' }).waitFor();
  await tags.getByRole('button', { name: 'Done' }).click();
  await differs.getByRole('button', { name: 'More for Guardian statues' }).click();
  await differs.getByRole('menuitem', { name: 'Send to 3D' }).click();
  const sendViews = differs.getByRole('dialog', { name: 'Send Guardian statues to 3D' });
  await sendViews
    .getByRole('group', { name: 'Provider' })
    .getByRole('button', { name: 'Tripo' })
    .click();
  await sendViews.getByText('Multi-view', { exact: true }).click();
  const differ = sendViews.getByText('Back looks different from the front', { exact: false });
  await differ.waitFor();
  await differ.scrollIntoViewIfNeeded();
  await differs.waitForTimeout(300);
  await shot(differs, 'refs-send-views-disagree');
  await differs.context().close();

  const batch = await open('&scenario=configured&lab=refs', '#/board');
  await batch.getByRole('checkbox', { name: 'Select Guardian statues' }).click();
  await batch.getByRole('checkbox', { name: 'Select Veil gate' }).click();
  await batch.getByRole('checkbox', { name: 'Select Rune obelisk' }).click();
  await batch
    .getByRole('toolbar', { name: 'Selected assets' })
    .getByRole('button', { name: 'Send to 3D' })
    .click();
  const many = batch.getByRole('dialog', { name: /^Send \d+ assets to 3D$/ });
  await many.getByText(/assets? ha(s|ve) ref warnings/).waitFor();
  await batch.waitForTimeout(300);
  await shot(batch, 'refs-batch-warnings');
  await many
    .getByRole('group', { name: 'Provider' })
    .getByRole('button', { name: 'Tripo' })
    .click();
  await many.getByRole('button', { name: /^Send \d+ to Tripo$/ }).click();
  await many.getByText(/with ref warnings\./).waitFor();
  await shot(batch, 'refs-batch-approval');
  await batch.context().close();
}
