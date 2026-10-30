import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SelectionBar } from '.';

function bar(overrides: Partial<Parameters<typeof SelectionBar>[0]> = {}) {
  const props = {
    count: 3,
    note: 'all checks pass',
    acceptable: 3,
    redoBlocker: null,
    onAccept: vi.fn(),
    onRedo: vi.fn(),
    onDiscard: vi.fn(),
    onClear: vi.fn(),
    ...overrides,
  };
  render(<SelectionBar {...props} />);
  return props;
}

describe('the inbox selection bar', () => {
  it('accepts, redoes, discards or clears the ticked models', async () => {
    const props = bar();
    expect(screen.getByRole('toolbar')).toHaveTextContent('3 selected · all checks pass');
    await userEvent.click(screen.getByRole('button', { name: 'Accept 3' }));
    await userEvent.click(screen.getByRole('button', { name: 'Redo 3' }));
    await userEvent.click(screen.getByRole('button', { name: 'Discard 3' }));
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(props.onAccept).toHaveBeenCalledOnce();
    expect(props.onRedo).toHaveBeenCalledOnce();
    expect(props.onDiscard).toHaveBeenCalledOnce();
    expect(props.onClear).toHaveBeenCalledOnce();
  });

  it('keeps Accept off when no file can be read and Redo off with its reason', async () => {
    const props = bar({ acceptable: 0, redoBlocker: 'Veil gate did not come from a Send to 3D' });
    const accept = screen.getByRole('button', { name: 'Accept 0' });
    expect(accept).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(accept);
    await userEvent.click(screen.getByRole('button', { name: 'Redo 3' }));
    expect(props.onAccept).not.toHaveBeenCalled();
    expect(props.onRedo).not.toHaveBeenCalled();
  });

  it('offers Accept and add to library in its wide form', async () => {
    const onAcceptToLibrary = vi.fn();
    bar({ wide: true, onAcceptToLibrary });
    await userEvent.click(screen.getByRole('button', { name: 'Accept and add to library' }));
    expect(onAcceptToLibrary).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'Discard' })).toBeInTheDocument();
  });
});
