import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LogEntry } from '@/ipc/client';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { RunLog } from '.';

const entry = (text: string, step: string | null = 'export'): LogEntry => ({
  runId: 'moon-island-run-24',
  step,
  stream: 'stdout',
  level: 'info',
  text,
  atMs: null,
});

const log = [entry('Blender 4.2.0'), entry('[export] wrote island.glb')];
const view = pipelineView('moon-island', '/w', runHistory(0), log);

afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe('the run log', () => {
  it('shows the lines and hides the noise when asked', async () => {
    render(<RunLog view={view} step="all" onStep={vi.fn()} />);
    expect(screen.getByText('Blender 4.2.0')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Hide noise' }));
    expect(screen.queryByText('Blender 4.2.0')).toBeNull();
    expect(localStorage.getItem('maquette.pipeline.hideNoise')).not.toBeNull();
  });

  it('hands the picked step to onStep', async () => {
    const onStep = vi.fn();
    render(<RunLog view={view} step="all" onStep={onStep} />);
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Show the log of' }),
      'Compress'
    );
    expect(onStep).toHaveBeenCalledWith('compress');
  });

  it('lists the step timings of the latest run without the skipped ones', async () => {
    render(<RunLog view={view} step="all" onStep={vi.fn()} />);
    await userEvent.click(screen.getByRole('tab', { name: 'Timings' }));
    expect(screen.getAllByText('cached')).toHaveLength(2);
    expect(screen.getByText('Capture posters')).toBeVisible();
    expect(screen.queryByText('Ship')).toBeNull();
  });

  it('says when a run kept no log', () => {
    render(<RunLog view={{ ...view, log: [] }} step="all" onStep={vi.fn()} />);
    expect(screen.getByText('No log kept for run 24. The next run streams here.')).toBeVisible();
  });
});
