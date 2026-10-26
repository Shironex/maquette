import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import LibraryDialogFrame from './LibraryDialogFrame';

describe('the library dialog frame', () => {
  it('submits from its form and closes on Escape', async () => {
    const onSubmit = vi.fn();
    const onClose = vi.fn();
    render(
      <LibraryDialogFrame
        id="t"
        title="Add to library"
        help="Copies the model"
        onClose={onClose}
        onSubmit={onSubmit}
        footer={<button type="submit">Go</button>}
      >
        <p>Body</p>
      </LibraryDialogFrame>
    );
    expect(screen.getByRole('dialog', { name: 'Add to library' })).toHaveTextContent(
      'Copies the model'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Go' }));
    expect(onSubmit).toHaveBeenCalled();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });
});
