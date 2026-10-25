import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ObjectChip } from '.';
import { tooltipText } from '@/test/tooltipText';

afterEach(cleanup);

describe('an object chip', () => {
  it('shows the notes as its title and hands out edit and remove', async () => {
    const onToggle = vi.fn();
    const onRemove = vi.fn();
    render(
      <ObjectChip
        name="Lantern"
        notes="Pink paper"
        editing={false}
        onToggle={onToggle}
        onRemove={onRemove}
      />
    );
    const edit = screen.getByRole('button', { name: 'Edit Lantern' });
    expect(await tooltipText(edit)).toBe('Notes for CodexPink paper');
    expect(edit).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(edit);
    expect(onToggle).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole('button', { name: 'Remove Lantern' }));
    expect(onRemove).toHaveBeenCalledTimes(1);
  });

  it('invites notes when there are none', async () => {
    render(
      <ObjectChip
        name="Bench"
        notes={undefined}
        editing={false}
        onToggle={vi.fn()}
        onRemove={vi.fn()}
      />
    );
    expect(await tooltipText(screen.getByRole('button', { name: 'Edit Bench' }))).toBe(
      'Add notes or rename'
    );
  });
});
