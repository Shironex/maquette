import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ObjectEditor } from '.';

afterEach(cleanup);

describe('the object editor', () => {
  it('saves the name and notes on Done', async () => {
    const onSave = vi.fn(() => true);
    render(<ObjectEditor name="Bench" notes="" onSave={onSave} onCancel={vi.fn()} />);
    const name = screen.getByRole('textbox', { name: 'Name' });
    await userEvent.clear(name);
    await userEvent.type(name, 'Park bench');
    await userEvent.type(screen.getByRole('textbox', { name: 'Notes for Codex' }), 'Wood');
    await userEvent.click(screen.getByRole('button', { name: 'Done' }));
    expect(onSave).toHaveBeenCalledWith({ name: 'Park bench', notes: 'Wood' });
  });

  it('says why a save was refused', async () => {
    render(<ObjectEditor name="Bench" notes="" onSave={() => false} onCancel={vi.fn()} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), '{Enter}');
    expect(screen.getByText('Another object already has that name.')).toBeVisible();
    await userEvent.clear(screen.getByRole('textbox', { name: 'Name' }));
    await userEvent.click(screen.getByRole('button', { name: 'Done' }));
    expect(screen.getByText('The object needs a name.')).toBeVisible();
  });

  it('saves on Cmd+Enter in the notes and cancels', async () => {
    const onSave = vi.fn(() => true);
    const onCancel = vi.fn();
    render(<ObjectEditor name="Bench" notes="Wood" onSave={onSave} onCancel={onCancel} />);
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'Notes for Codex' }), {
      key: 'Enter',
      metaKey: true,
    });
    expect(onSave).toHaveBeenCalledWith({ name: 'Bench', notes: 'Wood' });
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('blocks a name over the limit', async () => {
    render(<ObjectEditor name={'x'.repeat(61)} notes="" onSave={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByText('Keep the name to 60 characters.')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Done' })).toBeDisabled();
  });
});
