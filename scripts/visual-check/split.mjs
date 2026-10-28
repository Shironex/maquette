/**
 * Splits on a provider and Assemble one prop, for the parts set: the split
 * sheet's three ways (in Blender, Tripo with its confirm question, Meshy by
 * named parts), a Tripo split that lost its texture offering the bake back
 * and the bake done, and three Sigil gate parts assembled into one prop.
 * Mock mode's `?parts=demo` and `?parts=split` stand in for real models.
 */

const DEMO = '&scenario=configured&parts=demo';
const SPLIT = '&scenario=configured&parts=split';

async function partsTab(open, query, model) {
  const tab = await open(query, '#/inspector');
  await tab.getByRole('tab', { name: 'Parts' }).click();
  await tab.getByRole('combobox', { name: 'Selected model' }).selectOption({ label: model });
  return tab;
}

export async function splitShots({ open, shot }) {
  const tab = await partsTab(open, DEMO, 'Rune chest');
  await tab.getByRole('heading', { name: 'One mesh, no parts' }).waitFor();
  await tab.getByRole('button', { name: 'Split into parts...' }).click();
  const sheet = tab.getByRole('dialog', { name: 'Split Rune chest into parts' });
  await sheet.getByText(/v1 from Meshy/).waitFor();
  await shot(tab, 'parts-split-blender');

  await sheet.getByRole('button', { name: 'Tripo', exact: true }).click();
  await sheet.getByRole('button', { name: 'Semantic, beta' }).click();
  await sheet.getByText('Complete hidden parts').click();
  await sheet.getByText('About 140 credits').waitFor();
  await shot(tab, 'parts-split-tripo');
  await sheet.getByRole('button', { name: 'Split with Tripo' }).click();
  await tab.getByRole('dialog', { name: 'Split Rune chest with Tripo?' }).waitFor();
  await shot(tab, 'parts-split-tripo-confirm');
  await tab.getByRole('button', { name: 'Cancel' }).last().click();

  await sheet.getByRole('button', { name: 'Meshy', exact: true }).click();
  await sheet.getByRole('button', { name: 'By parts' }).click();
  const names = sheet.getByRole('textbox', { name: 'Add a part name' });
  await names.fill('lid, body, rune_lock');
  await names.press('Enter');
  await sheet.getByText('3 of 10').waitFor();
  await shot(tab, 'parts-split-meshy');
  await tab.context().close();

  const bake = await partsTab(open, SPLIT, 'Frost lantern');
  await bake.getByText('Tripo did not keep the texture').waitFor();
  await bake.waitForTimeout(1200);
  await shot(bake, 'parts-bake-offered');
  await bake.getByRole('button', { name: 'Bake back' }).click();
  await bake.getByText('Texture baked back as v4').waitFor();
  await shot(bake.getByRole('complementary', { name: 'Parts' }), 'parts-bake-done');
  await bake.context().close();

  const gate = await partsTab(open, DEMO, 'Sigil gate frame');
  await gate.getByRole('button', { name: 'Assemble a prop...' }).click();
  const assemble = gate.getByRole('dialog', { name: 'Assemble one prop' });
  for (const part of ['Sigil gate door', 'Sigil gate steps']) {
    await assemble.getByRole('combobox', { name: 'Add a part' }).selectOption({ label: part });
  }
  await assemble.getByRole('textbox', { name: 'Prop name' }).fill('sigil_gate');
  const rename = [
    ['Sigil gate frame', 'frame'],
    ['Sigil gate door', 'door_leaf'],
    ['Sigil gate steps', 'steps'],
  ];
  for (const [asset, node] of rename) {
    await assemble.getByRole('textbox', { name: `Node name for ${asset}` }).fill(node);
  }
  await assemble
    .getByRole('combobox', { name: 'Pivot of door_leaf' })
    .selectOption({ label: 'Left hinge' });
  await shot(gate, 'parts-assemble');
  await assemble.getByRole('button', { name: 'By numbers' }).click();
  const x = assemble.getByRole('textbox', { name: 'door_leaf x' });
  await x.fill('-0.6');
  await x.press('Enter');
  await shot(gate, 'parts-assemble-numbers');
  await assemble.getByRole('button', { name: 'Assemble', exact: true }).click();
  await assemble.waitFor({ state: 'detached' });
  await gate
    .getByRole('combobox', { name: 'Selected model' })
    .selectOption({ label: 'sigil_gate' });
  await gate.waitForTimeout(1500);
  await shot(gate, 'parts-assembled');
  await gate.context().close();
}
