import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SpringSheets from './SpringSheets';

describe('the spring sheets up close', () => {
  it('starts on the sheet opened and switches between the ones there are', async () => {
    const onClose = vi.fn();
    render(
      <SpringSheets
        sheets={{ candidates: 'found.webp', before: null, after: 'sway.webp' }}
        start="after"
        onClose={onClose}
      />
    );
    const zoom = screen.getByRole('dialog', { name: 'Spring sheets' });
    expect(within(zoom).getByRole('img', { name: 'The sway test sheet' })).toHaveAttribute(
      'src',
      'sway.webp'
    );
    expect(within(zoom).queryByRole('button', { name: 'Chains' })).toBeNull();
    await userEvent.click(within(zoom).getByRole('button', { name: 'Parts found' }));
    expect(within(zoom).getByRole('img', { name: 'The parts found sheet' })).toBeInTheDocument();
  });
});
