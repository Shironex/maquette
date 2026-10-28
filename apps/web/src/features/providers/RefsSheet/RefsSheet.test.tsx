import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';
import { clearMocks, openMockRefs } from '@/test/mock-ipc';
import { RefsSheet } from '.';

afterEach(() => {
  clearMocks();
  useRefsStore.setState({ sheet: null, view: null, error: null, busy: false });
  useProvidersStore.setState({ panel: null });
});

async function openSheet() {
  await openMockRefs();
  render(<RefsSheet />);
  await useRefsStore.getState().open('frostveil', 'guardian-statues');
  return screen.findByRole('dialog', { name: 'Guardian statues refs' });
}

describe('the refs sheet', () => {
  it('shows the four slots in send order, how many are filled and the Other tray', async () => {
    const sheet = await openSheet();
    expect(within(sheet).getAllByText('1 of 4 views')).not.toHaveLength(0);
    expect(within(sheet).getByRole('button', { name: /guardian_statues_front.png: Front/ }));
    expect(within(sheet).getAllByRole('button', { name: /Empty, draw it/ })).toHaveLength(3);
    expect(within(sheet).getByText('Kept with the asset, never sent as views')).toBeInTheDocument();
    expect(
      within(sheet).getByRole('button', { name: 'View of guardian_statues_crop.png: Other' })
    ).toBeInTheDocument();
  });

  it('swaps two refs when a taken view is picked from the chip', async () => {
    const sheet = await openSheet();
    await userEvent.click(
      within(sheet).getByRole('button', { name: 'View of guardian_statues_crop.png: Other' })
    );
    expect(screen.getByText(/picking a taken one swaps the two/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('menuitem', { name: 'Front' }));
    expect(
      await within(sheet).findByRole('button', { name: 'View of guardian_statues_crop.png: Front' })
    ).toBeInTheDocument();
    expect(
      within(sheet).getByRole('button', { name: 'View of guardian_statues_front.png: Other' })
    ).toBeInTheDocument();
  });

  it('flags a weak ref on its tile', async () => {
    const sheet = await openSheet();
    expect(await within(sheet).findByTitle('Busy background in guardian_statues_crop.png'));
  });

  it('hands over to Send to 3D', async () => {
    const sheet = await openSheet();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Send to 3D' }));
    expect(useRefsStore.getState().sheet).toBeNull();
    expect(useProvidersStore.getState().panel?.asset).toBe('guardian-statues');
  });
});
