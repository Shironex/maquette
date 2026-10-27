import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { MAX_BATCH } from '@/state/batch';
import SelectionBar from './SelectionBar';

describe('SelectionBar', () => {
  it('counts the selection and sends, removes or clears it', async () => {
    const onSend = vi.fn();
    const onRemove = vi.fn();
    const onClear = vi.fn();
    render(<SelectionBar count={1} onSend={onSend} onRemove={onRemove} onClear={onClear} />);
    const bar = screen.getByRole('toolbar', { name: 'Selected assets' });
    expect(bar).toHaveTextContent('1 asset selected');
    expect(bar).toHaveTextContent('Shift-click to add a range');
    await userEvent.click(screen.getByRole('button', { name: 'Send to 3D' }));
    await userEvent.click(screen.getByRole('button', { name: 'Remove...' }));
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(onSend).toHaveBeenCalledOnce();
    expect(onRemove).toHaveBeenCalledOnce();
    expect(onClear).toHaveBeenCalledOnce();
  });

  it('says only the first batch goes when the selection is over the cap', () => {
    render(
      <SelectionBar count={MAX_BATCH + 1} onSend={vi.fn()} onRemove={vi.fn()} onClear={vi.fn()} />
    );
    expect(screen.getByRole('toolbar', { name: 'Selected assets' })).toHaveTextContent(
      `${MAX_BATCH + 1} assets selected`
    );
    expect(screen.getByText(`The first ${MAX_BATCH} go in one batch`)).toBeInTheDocument();
  });
});
