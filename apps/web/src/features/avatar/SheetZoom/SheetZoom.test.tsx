import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SheetZoom from './SheetZoom';

describe('the sheet zoom', () => {
  it('switches between the shoulders and each whole sheet, and closes', async () => {
    const onClose = vi.fn();
    render(<SheetZoom before="/before.webp" after="/after.webp" mode="after" onClose={onClose} />);
    expect(screen.getByRole('img', { name: 'The after sheet' })).toHaveAttribute(
      'src',
      '/after.webp'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Shoulders' }));
    expect(
      screen.getByRole('img', { name: 'Before: the shoulders in four poses' })
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Hands' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(onClose).toHaveBeenCalled();
  });

  it('shows the hands row when the sheets have one', async () => {
    render(
      <SheetZoom before="/before.webp" after="/after.webp" mode="after" hands onClose={vi.fn()} />
    );
    await userEvent.click(screen.getByRole('button', { name: 'Hands' }));
    expect(
      screen.getByRole('img', { name: 'After: both hands from the thumb side and from above' })
    ).toBeInTheDocument();
  });
});
