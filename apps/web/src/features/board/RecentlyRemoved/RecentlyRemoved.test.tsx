import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import RecentlyRemoved from './RecentlyRemoved';

const entry = {
  id: 'elsheria',
  name: 'Elsheria',
  files: 2,
  removedAt: '2026-10-01T00:00:00Z',
} as never;

describe('RecentlyRemoved', () => {
  it('restores the picked asset and closes', async () => {
    const restoreOne = vi.fn();
    const onClose = vi.fn();
    render(
      <RecentlyRemoved
        entries={[entry]}
        restoring={null}
        restoreOne={restoreOne}
        onClose={onClose}
      />
    );
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Elsheria' }));
    expect(restoreOne).toHaveBeenCalledWith('elsheria');
    expect(onClose).toHaveBeenCalled();
  });

  it('holds the list while one is restoring', async () => {
    render(
      <RecentlyRemoved
        entries={[entry]}
        restoring="elsheria"
        restoreOne={() => {}}
        onClose={() => {}}
      />
    );
    expect(await screen.findByRole('menuitem', { name: 'Elsheria' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});
