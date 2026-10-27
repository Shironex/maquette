import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import { ProjectTopBar } from '.';
import { tooltipText } from '@/test/tooltipText';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

describe('the project top bar', () => {
  it('waits for the pipeline before it offers a run', async () => {
    await openMockWorkspace();
    render(<ProjectTopBar />);
    const run = screen.getByRole('button', { name: 'Run pipeline' });
    expect(run).toHaveAttribute('aria-disabled', 'true');
    expect(await tooltipText(run)).toBe('Loading the pipeline');
  });

  it('runs the pipeline and opens the Pipeline view', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('frostveil');
    useAppStore.setState({ view: 'board' });
    render(<ProjectTopBar />);
    const run = screen.getByRole('button', { name: 'Run pipeline' });
    expect(await tooltipText(run)).toBe('Runs every step, skipping cached ones');
    await userEvent.click(run);
    expect(useAppStore.getState().view).toBe('pipeline');
    expect(await screen.findByRole('button', { name: 'Run in progress' })).toBeVisible();
  });

  it('shows a game the model budget and its stages, as the Board and Lab', async () => {
    await openMockGame('hero');
    render(<ProjectTopBar />);
    expect(
      screen.getByRole('meter', { name: /^Game budget, \d+ of \d+ models over$/ })
    ).toBeVisible();
    for (const label of ['Concept', 'Refs', 'Tripo', 'Blender', 'Export', 'Ship']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    expect(screen.queryByText('Web')).toBeNull();
  });

  it('shows the view action beside Run pipeline', async () => {
    await openMockWorkspace();
    render(<ProjectTopBar extra={<button type="button">Open in browser</button>} />);
    expect(screen.getByRole('button', { name: 'Open in browser' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Run pipeline' })).toBeVisible();
  });
});
