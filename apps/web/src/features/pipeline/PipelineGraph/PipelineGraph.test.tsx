import { cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { pipelineView, runHistory } from '@/ipc/mock/pipeline-fixtures';
import { PipelineGraph } from '.';

const view = pipelineView('frostveil', '/w', runHistory(0), []);

afterEach(cleanup);

describe('the pipeline graph', () => {
  it('draws every step in order and runs from the one picked', async () => {
    const onRunFrom = vi.fn();
    render(<PipelineGraph view={view} busy={false} onRunFrom={onRunFrom} />);
    const graph = screen.getByLabelText('Pipeline steps');
    const runs = within(graph).getAllByRole('button', { name: /^Run from / });
    expect(runs.map(b => b.getAttribute('aria-label'))).toEqual(
      view.def.steps.map(s => `Run from ${s.name}`)
    );
    await userEvent.click(screen.getByRole('button', { name: 'Run from Compress' }));
    expect(onRunFrom).toHaveBeenCalledWith('compress');
  });

  it('holds the run buttons while a run is busy', () => {
    render(<PipelineGraph view={view} busy onRunFrom={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Run from Build scene' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});
