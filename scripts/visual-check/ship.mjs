/**
 * The website Ship settings for scripts/visual-check.mjs (ONLY=ship): the
 * Build only switch and its tooltip, the Ship settings view as a new
 * project has it (scene style, KTX2), a hand made site file held back,
 * the hand-built site setup (models style, WebP, a model set to never,
 * a smaller site file kept), the presets panel with the codec greyed out
 * for WebP, and the disabled reasons.
 */

export async function shipShots({ open, shot, hoverTip }) {
  const runner = await open('&scenario=idle', '#/pipeline');
  const buildOnly = runner.getByRole('checkbox', { name: 'Build only' });
  await buildOnly.waitFor();
  // The box itself is hidden for the custom mark; its label takes the pointer.
  const label = runner.getByText('Build only', { exact: true });
  await hoverTip(runner, label, 'ship-build-only-tip');
  await label.click();
  await hoverTip(
    runner,
    runner.getByRole('button', { name: 'Run pipeline' }),
    'ship-run-build-only'
  );
  await runner.getByRole('button', { name: 'Ship settings' }).click();
  await runner.getByRole('heading', { name: "Ship to Moon Island's site" }).waitFor();
  await runner.mouse.move(0, 0);
  await shot(runner, 'ship-settings-scene');
  await hoverTip(
    runner,
    runner.getByRole('combobox', { name: 'Ship shop_house.glb' }),
    'ship-choice-tip'
  );
  await runner.context().close();

  const built = await open('&scenario=idle&ship=hand-built', '#/pipeline');
  await built.getByRole('checkbox', { name: 'Build only' }).waitFor();
  await built.getByText(/Textures ship as WebP/).waitFor();
  await shot(built, 'ship-presets-webp');
  await hoverTip(
    built,
    built.getByRole('combobox', { name: /codec$/ }).first(),
    'ship-codec-disabled-reason'
  );
  await built.getByRole('button', { name: 'Ship settings' }).click();
  await built.getByRole('heading', { name: "Ship to Moon Island's site" }).waitFor();
  await built.mouse.move(0, 0);
  await shot(built, 'ship-settings-hand-built');
  await hoverTip(
    built,
    built.getByRole('button', { name: 'Save folders' }),
    'ship-folders-disabled-reason'
  );
  await built.getByRole('heading', { name: /Size report/ }).scrollIntoViewIfNeeded();
  await shot(built, 'ship-size-report');
  await built.context().close();
}
