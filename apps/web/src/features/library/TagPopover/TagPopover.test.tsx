import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import TagPopover from './TagPopover';

afterEach(() => clearMocks());

describe('the bulk tag popover', () => {
  it('shows the tags on some and puts a typed tag on every one', async () => {
    await openMockLibrary('scale');
    const items = (await commands.libraryGet()).items;
    const furniture = items.filter(i => i.entry.tags.includes('furniture')).slice(0, 2);
    const picked = [...furniture, items.find(i => i.entry.id === 'farm-table')!];
    const known = [...new Set(items.flatMap(i => i.entry.tags))];
    render(<TagPopover picked={picked} known={known} onClose={vi.fn()} />);
    const pop = screen.getByRole('dialog', { name: 'Tag 3 models' });
    expect(
      within(pop).getByRole('button', { name: 'Put furniture on all 3, it is on 2' })
    ).toBeVisible();
    expect(within(pop).getByText('No tag is on every one yet.')).toBeVisible();
    await userEvent.type(within(pop).getByRole('textbox', { name: 'Tag' }), 'barn{Enter}');
    await waitFor(async () => {
      const now = (await commands.libraryGet()).items;
      for (const p of picked) {
        expect(now.find(i => i.entry.id === p.entry.id)!.entry.tags).toContain('barn');
      }
    });
  });

  it('closes with Escape', async () => {
    await openMockLibrary('scale');
    const items = (await commands.libraryGet()).items.slice(0, 1);
    const onClose = vi.fn();
    render(<TagPopover picked={items} known={[]} onClose={onClose} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Tag' }), '{Escape}');
    expect(onClose).toHaveBeenCalled();
  });
});
