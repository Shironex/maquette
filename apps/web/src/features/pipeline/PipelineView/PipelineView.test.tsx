import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockGame, openMockWorkspace } from '@/test/mock-ipc';
import { PipelineView } from '.';

afterEach(() => {
  clearMocks();
  usePipelineStore.setState({ view: null });
});

describe('the Pipeline view', () => {
  it('drops the log filter and Ignore the cache on a project switch', async () => {
    await openMockWorkspace();
    await usePipelineStore.getState().load('moon-island');
    render(<PipelineView />);
    const filter = screen.getByRole('combobox', { name: 'Show the log of' }) as HTMLSelectElement;
    const step = usePipelineStore.getState().view!.def.steps.at(-1)!;
    await userEvent.selectOptions(filter, step.name);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Ignore the cache' }));
    expect(filter.value).toBe(step.id);

    await act(async () => {
      await useProjectsStore.getState().switchTo('rooftop-cinema');
      await usePipelineStore.getState().load('rooftop-cinema');
    });
    expect(
      (screen.getByRole('combobox', { name: 'Show the log of' }) as HTMLSelectElement).value
    ).toBe('all');
    expect(screen.getByRole('checkbox', { name: 'Ignore the cache' })).not.toBeChecked();
  });

  it('shows a game the model budget and the Export stage, not the scene MB', async () => {
    await openMockGame('hero');
    await usePipelineStore.getState().load('crypt-game');
    render(<PipelineView />);
    expect(
      screen.getByRole('meter', { name: /^Model budget, \d+ of \d+ models over$/ })
    ).toBeVisible();
    expect(screen.getByText('Export')).toBeInTheDocument();
    expect(screen.queryByText('Web')).toBeNull();
    expect(screen.queryByText(/ MB of /)).toBeNull();
  });
});
