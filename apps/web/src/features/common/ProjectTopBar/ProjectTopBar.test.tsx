import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ProjectTopBar } from '.';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

describe('the project top bar', () => {
  it('waits for the pipeline before it offers a run', async () => {
    await openMockWorkspace();
    render(<ProjectTopBar />);
    const run = screen.getByRole('button', { name: 'Run pipeline' });
    expect(run).toBeDisabled();
    expect(run).toHaveAttribute('title', 'Loading the pipeline');
  });

  it('runs the pipeline and opens the Pipeline view', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('moon-island');
    useAppStore.setState({ view: 'board' });
    render(<ProjectTopBar />);
    const run = screen.getByRole('button', { name: 'Run pipeline' });
    expect(run).toHaveAttribute('title', 'Runs every step, skipping cached ones');
    await userEvent.click(run);
    expect(useAppStore.getState().view).toBe('pipeline');
    expect(await screen.findByRole('button', { name: 'Run in progress' })).toBeVisible();
  });
});
