import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { RedoBar } from '.';

describe('the inbox redo bar', () => {
  it('redoes the ticked models or clears them', async () => {
    const onRedo = vi.fn();
    const onClear = vi.fn();
    render(<RedoBar count={2} blocker={null} onRedo={onRedo} onClear={onClear} />);
    expect(screen.getByRole('toolbar')).toHaveTextContent('2 models selected');
    await userEvent.click(screen.getByRole('button', { name: 'Redo 2' }));
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(onRedo).toHaveBeenCalledOnce();
    expect(onClear).toHaveBeenCalledOnce();
  });

  it('keeps Redo off while the selection cannot go as one batch', async () => {
    const onRedo = vi.fn();
    render(
      <RedoBar
        count={1}
        blocker="Sakura did not come from a Send to 3D"
        onRedo={onRedo}
        onClear={() => {}}
      />
    );
    const redo = screen.getByRole('button', { name: 'Redo 1' });
    expect(redo).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(redo);
    expect(onRedo).not.toHaveBeenCalled();
  });
});
