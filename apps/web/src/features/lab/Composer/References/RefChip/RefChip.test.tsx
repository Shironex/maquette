import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { RefChip } from '.';

afterEach(cleanup);

describe('a reference chip', () => {
  it('shows the image, or its name without one, and removes it', async () => {
    const onRemove = vi.fn();
    const { rerender } = render(<RefChip url="/a.png" name="a.png" onRemove={onRemove} />);
    expect(screen.getByRole('img', { name: 'a.png' })).toHaveAttribute('src', '/a.png');
    rerender(<RefChip url={null} name="a.png" onRemove={onRemove} />);
    expect(screen.queryByRole('img')).toBeNull();
    expect(screen.getByText('a.png')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Remove a.png' }));
    expect(onRemove).toHaveBeenCalledTimes(1);
  });
});
