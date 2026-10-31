import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import RecentPacks from './RecentPacks';

afterEach(() => clearMocks());

describe('recent packs', () => {
  it('opens a finished pack and goes on with one Maquette closed on', async () => {
    await openMockLibrary('scale');
    const recent = await commands.packRecent();
    const onOpen = vi.fn();
    const onResume = vi.fn();
    render(
      <RecentPacks
        recent={recent}
        current={null}
        busy={false}
        onOpen={onOpen}
        onResume={onResume}
      />
    );
    const rows = screen.getAllByRole('listitem');
    expect(
      within(rows[0]!).getByText('Maquette closed with 6 of 18 done, model 7 is next')
    ).toBeVisible();
    await userEvent.click(within(rows[0]!).getByRole('button', { name: 'Go on from 7' }));
    expect(onResume).toHaveBeenCalledWith(recent[0]!.id);
    await userEvent.click(within(rows[1]!).getByRole('button', { name: 'Open' }));
    expect(onOpen).toHaveBeenCalledWith(recent[1]!.id);
  });

  it('shows nothing without a pack', () => {
    const { container } = render(
      <RecentPacks recent={[]} current={null} busy={false} onOpen={vi.fn()} onResume={vi.fn()} />
    );
    expect(container).toBeEmptyDOMElement();
  });
});
