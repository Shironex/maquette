import { act, cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { frostveilStyleBoard } from '@/ipc/mock/style-board';
import { useLabStore } from '@/state/use-lab-store';
import { StyleBoardSheet } from '.';
import { draftChanged } from './StyleBoardSheet.hooks';

const board = frostveilStyleBoard();
const view: LabView = {
  project: 'frostveil',
  projectDir: '/w/frostveil',
  directions: [],
  jobs: [],
  refs: [],
  concurrency: 4,
  style: board,
};
const saved = {
  line: board.line,
  palette: [...board.palette],
  images: board.images.map(i => i.file),
};
const lab = useLabStore.getState();

afterEach(() => {
  cleanup();
  useLabStore.setState(lab, true);
});

describe('the style board sheet', () => {
  it('knows when the draft differs from the board', () => {
    expect(draftChanged(saved, view)).toBe(false);
    expect(draftChanged({ ...saved, line: `  ${board.line}  ` }, view)).toBe(false);
    expect(draftChanged({ ...saved, palette: [] }, view)).toBe(true);
    expect(draftChanged({ ...saved, images: saved.images.slice(1) }, view)).toBe(true);
  });

  it('clears the draft and saves only once something changed', async () => {
    const saveStyle = vi.fn(async () => true);
    useLabStore.setState({ view, style: saved, saveStyle });
    const { rerender } = render(<StyleBoardSheet draft={saved} view={view} />);
    expect(screen.getByRole('button', { name: 'Save board' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    const cleared = useLabStore.getState().style!;
    expect(cleared).toEqual({ line: '', palette: [], images: [] });
    act(() => rerender(<StyleBoardSheet draft={cleared} view={view} />));
    await userEvent.click(screen.getByRole('button', { name: 'Save board' }));
    expect(saveStyle).toHaveBeenCalledTimes(1);
  });

  it('closes from the backdrop and the Close button', async () => {
    const closeStyle = vi.fn();
    useLabStore.setState({ view, style: saved, closeStyle });
    render(<StyleBoardSheet draft={saved} view={view} />);
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(closeStyle).toHaveBeenCalledTimes(2);
  });

  it('lets Escape close the Add menu first, then the dialog', async () => {
    const closeStyle = vi.fn();
    useLabStore.setState({ view, style: saved, closeStyle });
    render(<StyleBoardSheet draft={saved} view={view} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add style image' }));
    expect(screen.getByRole('menu', { name: 'Add a style image' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(closeStyle).not.toHaveBeenCalled();
    await userEvent.keyboard('{Escape}');
    expect(closeStyle).toHaveBeenCalledTimes(1);
  });
});
