import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import LinkDialog from './LinkDialog';

describe('LinkDialog', () => {
  it('adds https to a bare address and hands back the title on Enter', async () => {
    const onAdd = vi.fn();
    render(<LinkDialog onAdd={onAdd} onClose={vi.fn()} />);
    const address = screen.getByRole('textbox', { name: 'Address' });
    expect(address).toHaveFocus();
    expect(screen.getByRole('button', { name: 'Add link' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.type(address, 'example.com/frost');
    await userEvent.type(screen.getByRole('textbox', { name: 'Title' }), ' Frost shaders {Enter}');
    expect(onAdd).toHaveBeenCalledWith('https://example.com/frost', 'Frost shaders');
  });

  it('warns about something that is not a web address', async () => {
    const onAdd = vi.fn();
    render(<LinkDialog onAdd={onAdd} onClose={vi.fn()} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Address' }), 'javascript:alert(1)');
    await userEvent.tab();
    expect(
      screen.getByText('Type a web address, such as example.com/article.')
    ).toBeInTheDocument();
    await userEvent.keyboard('{Enter}');
    expect(onAdd).not.toHaveBeenCalled();
  });
});
