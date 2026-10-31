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
  await sheet.getByRole('radio', { name: /Re-texture with Meshy/ }).click();
  await sheet.getByRole('textbox', { name: 'Texture prompt' }).fill('old oak');
  await shot(tab, 'parts-split-retexture');
  await sheet.getByRole('button', { name: 'Split with Meshy' }).click();
  await tab.getByText(/You start it from the Parts tab once you accept the parts/).waitFor();
  await shot(tab, 'parts-split-retexture-confirm');
  await tab.context().close();

  // A disabled way is dimmed and says why on the sheet.
  const blocked = await partsTab(open, DEMO, 'Sigil gate frame');
  await blocked.getByRole('button', { name: 'Split into parts...' }).click();
  await blocked
    .getByRole('dialog', { name: /^Split Sigil gate frame/ })
    .getByText(/Meshy splits only models it made/)
    .waitFor();
  await shot(blocked, 'parts-split-disabled-way');
  await blocked.context().close();

  // A re-texture that came back as one mesh says so in the Inbox.
  const lost = await open('&scenario=configured&inbox=retexture&parts=retexture', '#/inbox');
  await lost.getByText('Fern clump').first().click();
  await lost.getByText('Parts gone: v2 has 35, this is one mesh').waitFor();
  await shot(lost, 'inbox-retexture-parts-gone');

  // Accepting a split with a re-texture chosen links to the offer.
  await lost.getByText('Frost lantern parts').first().click();
  await lost.getByRole('button', { name: /^Accept to Blender/ }).click();
  await lost.getByRole('button', { name: 'Open Parts tab' }).waitFor();
  await shot(lost, 'parts-retexture-accepted');
  await lost.getByRole('button', { name: 'Open Parts tab' }).click();
  await lost.getByText('v3 is grey for now').waitFor();
  await shot(lost, 'parts-retexture-offer');
  await lost.context().close();

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
  await assemble.getByRole('button', { name: 'Move in the viewer' }).click();
  await assemble
    .getByRole('combobox', { name: 'Part the gizmo moves' })
    .selectOption({ label: 'door_leaf' });
  await assemble.getByRole('button', { name: 'Turn' }).click();
  const stage = assemble.getByLabel('Assembly');
  await stage.scrollIntoViewIfNeeded();
  await gate.waitForTimeout(2500);
  await shot(gate, 'parts-assemble-viewer');
  await assemble.getByRole('button', { name: 'Assemble', exact: true }).click();
  await assemble.waitFor({ state: 'detached' });
  await gate
    .getByRole('combobox', { name: 'Selected model' })
    .selectOption({ label: 'sigil_gate' });
  await gate.waitForTimeout(1500);
  await shot(gate, 'parts-assembled');
  await gate.context().close();

  const grey = await partsTab(open, '&scenario=configured&parts=grey', 'Frost lantern');
  await grey.getByText('v3 is grey for now').waitFor();
  await grey.waitForTimeout(1500);
  await shot(grey, 'parts-meshy-grey');
  await grey.getByRole('button', { name: 'Re-texture...' }).click();
  await grey.getByRole('dialog', { name: 'Re-texture with Meshy?' }).waitFor();
  await shot(grey, 'parts-meshy-retexture-ask');
  await grey.context().close();
}
