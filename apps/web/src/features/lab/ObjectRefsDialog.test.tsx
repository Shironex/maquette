import { render, screen, waitFor, within } from '@testing-library/react';
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

  it('keeps a long name whole, with a counter, and blocks Make until it is short', async () => {
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
    const long = 'Finished island in painted stone and moss with no wireframe lines and no rocks';
    await userEvent.type(field, `${long}{Enter}`);
    expect(field).toHaveValue(long);
    expect(screen.getByRole('alert')).toHaveTextContent(`${long.length} of 60 characters`);
    expect(useLabStore.getState().objects?.objects).toEqual([]);
    expect(screen.getByRole('button', { name: 'Make object refs' })).toBeDisabled();

    await userEvent.clear(field);
    await userEvent.type(field, 'Island{Enter}');
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('sends notes with an object but names the asset after the object only', async () => {
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
    await userEvent.type(field, 'island,bench{Enter}');
    await userEvent.click(screen.getByRole('button', { name: 'Edit Island' }));
    const editor = screen.getByRole('group', { name: 'Edit Island' });
    const name = within(editor).getByRole('textbox', { name: 'Name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Moon island');
    await userEvent.type(
      within(editor).getByRole('textbox', { name: 'Notes for Codex' }),
      'Painted stone and moss, no wireframe lines'
    );
    await userEvent.click(within(editor).getByRole('button', { name: 'Done' }));
    expect(useLabStore.getState().objects).toMatchObject({
      objects: ['Moon island', 'Bench'],
      notes: { 'Moon island': 'Painted stone and moss, no wireframe lines' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Remove Moon island' }));
    expect(useLabStore.getState().objects?.notes).toEqual({});
  });

  it('explains the notes with a help mark, and a tip once there is a chip that stays dismissed', async () => {
    window.localStorage.clear();
    await openMockWorkspace();
    await useLabStore.getState().load('moon-island');
    const open = () =>
      useLabStore.getState().openObjects({
        project: 'moon-island',
        source: 'concepts/moon-island-porch/moon-island-porch_01.png',
        sourcePath: '/x/moon-island-porch_01.png',
        sourceName: 'Moon island porch',
      });
    const view = render(<ObjectRefsDialog />);
    open();
    const field = await screen.findByRole('textbox', { name: 'Objects' });
    expect(screen.getByRole('button', { name: 'About objects' })).toBeInTheDocument();
    expect(screen.getByRole('tooltip')).toHaveTextContent('never become the asset name');
    expect(screen.queryByRole('note')).toBeNull();

    await userEvent.type(field, 'bench{Enter}');
    expect(screen.getByRole('note')).toHaveTextContent('click an object to add notes');
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss tip' }));
    expect(screen.queryByRole('note')).toBeNull();

    // A later open does not bring it back.
    view.unmount();
    useLabStore.getState().closeObjects();
    render(<ObjectRefsDialog />);
    open();
    await userEvent.type(await screen.findByRole('textbox', { name: 'Objects' }), 'lamp{Enter}');
    expect(screen.queryByRole('note')).toBeNull();
    window.localStorage.clear();
  });
});
