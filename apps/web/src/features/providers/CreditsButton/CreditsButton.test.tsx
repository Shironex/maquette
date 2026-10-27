import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useCreditsStore } from '@/state/use-credits-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { CreditsButton } from '.';

afterEach(() => {
  clearMocks();
  useCreditsStore.setState({ open: false, ledger: null, balances: null });
});

describe('the Credits button', () => {
  it('closes what it sits in, then opens the sheet for the project', async () => {
    await openMockWorkspace();
    const onOpen = vi.fn();
    render(<CreditsButton project="frostveil" onOpen={onOpen} />);
    await userEvent.click(screen.getByRole('button', { name: 'Credits' }));
    expect(onOpen).toHaveBeenCalledOnce();
    expect(useCreditsStore.getState()).toMatchObject({
      open: true,
      filter: { project: 'frostveil' },
    });
  });
});
