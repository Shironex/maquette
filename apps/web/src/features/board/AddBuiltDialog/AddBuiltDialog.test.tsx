import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useBuiltStore } from '@/state/use-built-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import AddBuiltDialog from './AddBuiltDialog';

beforeEach(async () => {
  await openMockWorkspace();
  await useBuiltStore.getState().load('frostveil');
});

afterEach(() => {
  clearMocks();
  useBuiltStore.setState({ project: null, items: [], adding: false });
});

describe('AddBuiltDialog', () => {
  it('names the asset after the picked script and adds it with its helpers', async () => {
    const onClose = vi.fn();
    render(<AddBuiltDialog project="frostveil" onClose={onClose} />);
    const dialog = screen.getByRole('dialog', { name: 'Add built asset' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Choose...' }));
    expect(within(dialog).getByRole('textbox', { name: 'Name' })).toHaveValue('Desk');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add files...' }));
    expect(within(dialog).getByRole('list', { name: 'Helpers' })).toHaveTextContent('common.py');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add and build' }));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    await vi.waitFor(() =>
      expect(useBuiltStore.getState().items.map(b => b.asset)).toContain('desk')
    );
    expect(useBuiltStore.getState().items[0]?.source.helpers).toEqual(['built/common.py']);
  });

  it('keeps a typed name and asks for a script first', async () => {
    render(<AddBuiltDialog project="frostveil" onClose={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add and build' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Pick the script that builds the model.');
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Writing desk');
    await userEvent.click(screen.getByRole('button', { name: 'Choose...' }));
    expect(screen.getByRole('textbox', { name: 'Name' })).toHaveValue('Writing desk');
  });

  it('shows why a script is refused', async () => {
    render(<AddBuiltDialog project="frostveil" onClose={vi.fn()} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Script' }), '/tmp/notes.txt');
    await userEvent.click(screen.getByRole('button', { name: 'Add and build' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('notes.txt is not a Python script.');
  });
});
