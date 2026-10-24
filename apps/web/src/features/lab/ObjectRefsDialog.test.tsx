import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useLabStore } from '@/state/lab-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ObjectRefsDialog } from './ObjectRefsDialog';

afterEach(() => {
  clearMocks();
  useLabStore.setState({ view: null, selected: null, objects: null, busy: null });
});

describe('Make object refs', () => {
  it('takes a list of objects and queues one titled job each in a new direction', async () => {
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    render(<ObjectRefsDialog />);
    useLabStore.getState().openObjects({
      project: 'moon-island',
      source: 'concepts/moon-island-porch/moon-island-porch_01.png',
      sourcePath: '/x/moon-island-porch_01.png',
      sourceName: 'Moon island porch',
    });
    const field = await screen.findByRole('textbox', { name: 'Objects' });
    await userEvent.type(field, 'bench,vending machine{Enter}');
    await userEvent.type(field, 'paper lantern');
    expect(screen.getByRole('button', { name: 'Remove Vending machine' })).toBeInTheDocument();
    expect(screen.getByText('Moon island porch objects')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Make 3 object refs' }));

    await waitFor(() => expect(useLabStore.getState().objects).toBeNull());
    const { view, selected } = useLabStore.getState();
    expect(selected).toBe('moon-island-porch-objects');
    const direction = view!.directions.find(d => d.id === selected)!;
    expect(direction).toMatchObject({
      aspect: '1:1',
      kind: 'prop',
      source: 'concepts/moon-island-porch/moon-island-porch_01.png',
    });
    expect(view!.jobs.filter(j => j.direction === selected).map(j => j.title)).toEqual([
      'Bench',
      'Vending machine',
      'Paper lantern',
    ]);
  });
});
