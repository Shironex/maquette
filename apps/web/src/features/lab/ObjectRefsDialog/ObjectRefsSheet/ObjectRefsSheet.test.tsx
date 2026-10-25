import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useLabStore, type ObjectsDraft } from '@/state/use-lab-store';
import { ObjectRefsSheet } from '.';

const draft: ObjectsDraft = {
  project: 'moon-island',
  source: 'concepts/moon-island-porch/moon-island-porch_01.png',
  sourcePath: '/x/moon-island-porch_01.png',
  sourceName: 'Moon island porch',
  objects: ['Bench', 'Lantern'],
  notes: { Lantern: 'Pink paper' },
  aspect: '1:1',
  kind: 'prop',
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
  localStorage.clear();
});

describe('the object refs sheet', () => {
  it('shows the source, the objects and where the refs land', () => {
    render(<ObjectRefsSheet objects={draft} />);
    expect(screen.getByRole('dialog', { name: 'Make object refs' })).toBeVisible();
    expect(screen.getByText(draft.source)).toBeVisible();
    expect(screen.getByText('2 of 12')).toBeVisible();
    expect(screen.getByText('Moon island porch objects')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Make 2 object refs' })).toBeEnabled();
  });

  it('adds an object on a comma and drops the last one on Backspace', async () => {
    const setObjects = vi.fn();
    useLabStore.setState({ setObjects });
    render(<ObjectRefsSheet objects={draft} />);
    const field = screen.getByRole('textbox', { name: 'Objects' });
    await userEvent.type(field, 'desk lamp,');
    expect(setObjects).toHaveBeenLastCalledWith({ objects: ['Bench', 'Lantern', 'Desk lamp'] });
    await userEvent.type(field, '{Backspace}');
    expect(setObjects).toHaveBeenLastCalledWith({ objects: ['Bench'], notes: {} });
  });

  it('switches the kind and closes from Cancel', async () => {
    const setObjects = vi.fn();
    const closeObjects = vi.fn();
    useLabStore.setState({ setObjects, closeObjects });
    render(<ObjectRefsSheet objects={draft} />);
    await userEvent.click(screen.getByRole('button', { name: 'Characters' }));
    expect(setObjects).toHaveBeenCalledWith({ kind: 'character' });
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(closeObjects).toHaveBeenCalledTimes(1);
  });

  it('opens the editor for the object clicked', async () => {
    render(<ObjectRefsSheet objects={draft} />);
    await userEvent.click(screen.getByRole('button', { name: 'Edit Lantern' }));
    expect(screen.getByRole('group', { name: 'Edit Lantern' })).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Notes for Codex' })).toHaveValue('Pink paper');
  });
});
