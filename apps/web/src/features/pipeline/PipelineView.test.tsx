import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePipelineStore } from '@/state/pipeline-store';
import { useProjectsStore } from '@/state/projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PipelineView } from './PipelineView';

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
});
