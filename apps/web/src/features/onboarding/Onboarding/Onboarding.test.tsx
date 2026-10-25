import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { Onboarding } from '.';

afterEach(() => clearMocks());

describe('onboarding', () => {
  it('opens on the welcome and walks to the workspace step and back', async () => {
    await openMockWorkspace('first-run');
    render(<Onboarding />);
    expect(screen.getByRole('heading', { level: 1, name: 'maquette' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Set up' }));
    expect(
      screen.getByRole('heading', { level: 1, name: 'Pick a home for your projects' })
    ).toBeVisible();
    expect(screen.getByText('Step 2 of 7')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(screen.getByRole('button', { name: 'Set up' })).toBeVisible();
  });

  it('goes back to a done step from the stepper', async () => {
    await openMockWorkspace('first-run');
    render(<Onboarding />);
    await userEvent.click(screen.getByRole('button', { name: 'Set up' }));
    const stepper = screen.getByRole('list', { name: 'Setup steps' });
    await userEvent.click(within(stepper).getByRole('button', { name: /Welcome/ }));
    expect(screen.getByRole('button', { name: 'Set up' })).toBeVisible();
  });
});
