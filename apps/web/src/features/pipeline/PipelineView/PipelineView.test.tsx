import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProjectsStore } from '@/state/use-projects-store';
import {
  clearMocks,
  openMockGame,
  openMockPipeline,
  openMockShip,
  openMockWorkspace,
} from '@/test/mock-ipc';
import { PipelineView } from '.';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

describe('the Pipeline view', () => {
  it('asks before a run with no site repo, and Build only does not', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    const workspace = useAppStore.getState().workspace!;
    act(() =>
      useAppStore.setState({
        workspace: { ...workspace, project: { ...workspace.project!, siteRepo: null } },
      })
    );
    const view = usePipelineStore.getState().view!;
    const steps = view.def.steps.map(s =>
      s.id === 'ship' ? { ...s, args: ['--site', '{siteRepo}'] } : s
    );
    act(() => usePipelineStore.setState({ view: { ...view, def: { ...view.def, steps } } }));
    render(<PipelineView />);
    await userEvent.click(screen.getByRole('button', { name: 'Run pipeline' }));
    expect(screen.getByText(/builds everything and ships nothing/)).toBeVisible();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByText(/builds everything and ships nothing/)).toBeNull();
  });

  it('says a finished run shipped nothing, with the reason', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    const view = usePipelineStore.getState().view!;
    const reason = 'No site repo is set for this project, so nothing was shipped.';
    const last = { ...view.runs[0]!, state: 'done' as const, notShipped: reason };
    act(() => usePipelineStore.setState({ view: { ...view, active: null, runs: [last] } }));
    render(<PipelineView />);
    expect(screen.getByText(`Run ${last.number} finished, but nothing shipped`)).toBeVisible();
    expect(screen.getByText(reason)).toBeVisible();
  });

  it('drops the log filter, Build only and Ignore the cache on a project switch', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    render(<PipelineView />);
    const filter = screen.getByRole('combobox', { name: 'Show the log of' }) as HTMLSelectElement;
    const step = usePipelineStore.getState().view!.def.steps.at(-1)!;
    await userEvent.selectOptions(filter, step.name);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Ignore the cache' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Build only' }));
    expect(filter.value).toBe(step.id);

    await act(async () => {
      await useProjectsStore.getState().switchTo('frost-cathedral');
      await usePipelineStore.getState().load('frost-cathedral');
    });
    expect(
      (screen.getByRole('combobox', { name: 'Show the log of' }) as HTMLSelectElement).value
    ).toBe('all');
    expect(screen.getByRole('checkbox', { name: 'Ignore the cache' })).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Build only' })).not.toBeChecked();
  });

  it("opens a website's Ship settings and greys the codec out for WebP", async () => {
    await openMockShip('hand-built');
    await usePipelineStore.getState().load('frostveil');
    render(<PipelineView />);
    expect(await screen.findByText(/Textures ship as WebP/)).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Crystal lantern codec' })).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'Ship settings' }));
    expect(
      await screen.findByRole('heading', { level: 1, name: "Ship to Frostveil's site" })
    ).toBeInTheDocument();
  });

  it('shows a game the model budget and the Export stage, not the scene MB', async () => {
    await openMockGame('hero');
    await usePipelineStore.getState().load('crypt-game');
    render(<PipelineView />);
    expect(
      screen.getByRole('meter', { name: /^Game budget, \d+ of \d+ models over$/ })
    ).toBeVisible();
    expect(screen.getByText('Export')).toBeInTheDocument();
    expect(screen.queryByText('Web')).toBeNull();
    expect(screen.queryByText(/ MB of /)).toBeNull();
  });

  it('folds the Baked light card when every model skips it, and lists results in the presets', async () => {
    await openMockPipeline('many');
    await usePipelineStore.getState().load('frostveil');
    render(<PipelineView />);
    expect(
      screen.getByText(/^All \d+ models skip the bake, so this step bakes nothing\.$/)
    ).toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: 'Lightmap size' })).toBeNull();
    expect(screen.getByRole('region', { name: 'Generated artifacts' })).toBeVisible();
    expect(screen.getByText(/^24 models · /)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Size report' }));
    expect(
      await screen.findByRole('heading', { level: 1, name: "Ship to Frostveil's site" })
    ).toBeInTheDocument();
  });

  it('keeps the Baked light card open while some model bakes', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    render(<PipelineView />);
    expect(screen.getByRole('combobox', { name: 'Lightmap size' })).toBeInTheDocument();
  });
});
