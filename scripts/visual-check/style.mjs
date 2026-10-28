/**
 * The Lab's style board for scripts/visual-check.mjs (ONLY=style), on
 * Frostveil with its board (`?style=board`): the composer with the project
 * style on and off, a direction's chip for the revision it carried and for
 * one made without it, Add to style board on a concept, the style board
 * dialog with its Add menu and a palette being typed, the switch in Make
 * object refs, and a project without a board offering to set one up.
 */

export async function styleShots({ open, shot, hoverTip }) {
  const lab = await open('&scenario=configured&style=board&conceptMs=60000', '#/lab');
  await lab.getByRole('heading', { name: 'Frozen lake arena' }).waitFor();
  await lab.getByText('Style r3').waitFor();
  await shot(lab, 'style-composer-on');
  await hoverTip(
    lab,
    lab.getByRole('button', { name: 'Add Concept 02 to the style board' }),
    'style-add-from-grid'
  );
  await lab.getByRole('switch', { name: /^Use the project style/ }).click({ force: true });
  await lab.getByText('This direction keeps its own look').waitFor();
  await shot(lab, 'style-composer-off');

  await lab.getByRole('button', { name: /^Elsheria sheets/ }).click();
  await lab.getByRole('heading', { name: 'Elsheria sheets' }).waitFor();
  await lab.getByText('Own style').waitFor();
  await shot(lab, 'style-direction-own');

  await lab.getByRole('button', { name: 'Open the style board' }).click();
  const dialog = lab.getByRole('dialog', { name: 'Style board' });
  await dialog.waitFor();
  await lab.waitForTimeout(300);
  await shot(lab, 'style-board-dialog');
  await dialog.getByRole('button', { name: 'Add style image' }).click();
  await lab.getByRole('menu', { name: 'Add a style image' }).waitFor();
  await shot(lab, 'style-board-add-menu');
  await lab.keyboard.press('Escape');
  await dialog.getByRole('textbox', { name: 'Colour', exact: true }).fill('#c7b8ff');
  await dialog
    .getByRole('textbox', { name: 'Style line' })
    .fill('frosted glass, violet rune light');
  await shot(lab, 'style-board-editing');
  await dialog.getByRole('button', { name: 'Close' }).click();
  await lab.context().close();

  const objects = await open('&scenario=configured&style=board&conceptMs=60000', '#/lab');
  await objects.getByRole('heading', { name: 'Frozen lake arena' }).waitFor();
  await objects.getByRole('button', { name: 'Make object refs from Concept 01' }).click();
  const field = objects.getByRole('textbox', { name: 'Objects' });
  await field.fill('rune pillar, crystal lantern');
  await field.press('Enter');
  await objects.getByRole('dialog', { name: 'Make object refs' }).waitFor();
  await shot(objects, 'style-object-refs');
  await objects.context().close();

  const empty = await open('&scenario=configured&conceptMs=60000', '#/lab');
  await empty.getByRole('heading', { name: 'Frozen lake arena' }).waitFor();
  await empty.getByRole('button', { name: 'Set up' }).click();
  await empty.getByRole('dialog', { name: 'Style board' }).waitFor();
  await empty.waitForTimeout(300);
  await shot(empty, 'style-board-empty');
  await empty.context().close();
}
