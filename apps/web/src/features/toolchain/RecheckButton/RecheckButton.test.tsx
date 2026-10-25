import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { RecheckButton } from '.';

const initial = useAppStore.getState();

afterEach(() => {
  cleanup();
  useAppStore.setState(initial, true);
});

describe('the re-check button', () => {
  it('runs the detection again', async () => {
    const detectTools = vi.fn(async () => {});
    useAppStore.setState({ detectTools });
    render(<RecheckButton />);
    await userEvent.click(screen.getByRole('button', { name: /Re-check all/ }));
    expect(detectTools).toHaveBeenCalledTimes(1);
  });
});
