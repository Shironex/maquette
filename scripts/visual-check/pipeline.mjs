/**
 * The Pipeline runner for scripts/visual-check.mjs (ONLY=pipeline), at
 * 1440x900: a 24 model project after a build only run (the Baked light card
 * folded because every model skips the bake, the presets sorted by shipped
 * size, a data map line open, the bake settings opened again), the short
 * Frostveil project, Preview on site running with Stop beside it and the
 * status bar's dev server menu, Stop from there, the Page view showing who
 * started the server, the Size report link, a
 * failed run, the log, idle, running and the starter scripts banner.
 */

export async function pipelineShots({ open, shot, hoverTip }) {
  const many = await open('&scenario=idle&pipeline=many', '#/pipeline');
  await many.getByRole('heading', { name: 'Pipeline runner' }).waitFor();
  await many.getByText(/^24 models · /).waitFor();
  await shot(many, 'pipeline-runner-many');
  await many
    .getByRole('combobox', { name: 'Sort the models by' })
    .selectOption({ label: 'By shipped size' });
  await many.getByRole('button', { name: 'Texture sizes per map for Veil platform' }).click();
  await shot(many, 'pipeline-presets-sorted');
  await many.getByRole('button', { name: 'Bake settings' }).click();
  await many.getByRole('combobox', { name: 'Lightmap size' }).waitFor();
  await many.mouse.move(0, 0);
  await shot(many, 'pipeline-bake-open');
  await hoverTip(
    many,
    many.getByRole('separator', { name: 'Resize the compression presets' }),
    'pipeline-resize-tip'
  );
  const handle = many.getByRole('separator', { name: 'Resize the compression presets' });
  await handle.focus();
  for (let i = 0; i < 6; i += 1) await many.keyboard.press('ArrowLeft');
  await many.mouse.move(0, 0);
  await shot(many, 'pipeline-presets-wide');
  await many.getByRole('button', { name: 'Size report' }).click();
  await many.getByRole('heading', { name: /Size report/ }).waitFor();
  await shot(many, 'pipeline-size-report-link');
  await many.context().close();

  const short = await open('&scenario=idle', '#/pipeline');
  await short.getByRole('heading', { name: 'Pipeline runner' }).waitFor();
  await shot(short, 'pipeline-runner-short');
  await hoverTip(
    short,
    short.getByRole('button', { name: 'Preview on site' }),
    'pipeline-preview-tip'
  );
  await short.getByRole('button', { name: 'Preview on site' }).click();
  await short.getByRole('button', { name: 'Open preview' }).waitFor();
  await short.getByRole('button', { name: 'Dismiss' }).click();
  await short.mouse.move(0, 0);
  await shot(short, 'pipeline-preview-running');
  await hoverTip(
    short,
    short.getByRole('button', { name: "Stop the site's dev server" }),
    'pipeline-preview-stop-tip'
  );
  await short.getByRole('button', { name: /^Site dev page on/ }).click();
  await short.getByRole('menu', { name: 'Site dev server' }).waitFor();
  await shot(short, 'pipeline-status-menu');
  await short.getByRole('menuitem', { name: 'Stop the dev server' }).click();
  await short.getByRole('button', { name: 'Preview on site' }).waitFor();
  await short.mouse.move(0, 0);
  await shot(short, 'pipeline-preview-stopped');
  await short.context().close();

  // Started by Preview on site, the Page view says so and stops the same server.
  const shared = await open('&scenario=idle', '#/pipeline');
  await shared.getByRole('button', { name: 'Preview on site' }).click();
  await shared.getByRole('button', { name: 'Open preview' }).waitFor();
  await shared.getByRole('button', { name: 'Dismiss' }).click();
  await shared.getByRole('button', { name: /^Site dev page on/ }).click();
  await shared.getByRole('menuitem', { name: 'Show in the Page view' }).click();
  await shared.getByText('started by Preview on site').waitFor();
  await shared.getByRole('button', { name: /^Jump to stop 2, / }).waitFor();
  await shared.mouse.move(0, 0);
  await shot(shared, 'page-started-by-preview');
  await shared.context().close();

  const failed = await open('&scenario=configured', '#/pipeline');
  await failed.getByText('Run 24 failed at Capture posters').first().waitFor();
  await shot(failed, 'pipeline-failed');
  await failed.getByText(/^New starter scripts for /).waitFor();
  await shot(failed, 'pipeline-starters');
  await failed.context().close();

  // A tall window, so the whole log shows.
  const logs = await open('&scenario=configured', '#/pipeline', { width: 1440, height: 1500 });
  await logs.getByRole('log').getByText('Traceback').waitFor();
  await shot(logs.getByRole('log'), 'pipeline-log');
  await logs.getByText('Hide noise').click();
  await shot(logs.getByRole('log'), 'pipeline-log-quiet');
  await logs.context().close();

  const idle = await open('&scenario=idle', '#/pipeline');
  await idle.getByRole('heading', { name: 'Pipeline runner' }).waitFor();
  await shot(idle, 'pipeline-idle');
  await idle.context().close();

  const running = await open('&scenario=running&pace=60', '#/pipeline');
  await running.getByText('Run 25 · step 3 of 6').first().waitFor();
  await running.waitForTimeout(1200);
  await shot(running, 'pipeline-running');
  await running.context().close();
}
