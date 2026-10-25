import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useOnboarding } from '../../Onboarding/Onboarding.hooks';
import { AgentsStep } from '.';

afterEach(() => clearMocks());

function Harness() {
  const api = useOnboarding();
  return (
    <>
      <output aria-label="step">{api.step}</output>
      <AgentsStep api={api} />
    </>
  );
}

describe('the agents step', () => {
  it('turns the built-in MCP server on and off', async () => {
    await openMockWorkspace('first-run');
    render(<Harness />);
    expect(screen.getByRole('region', { name: 'Built-in MCP server' })).toBeVisible();
    expect(screen.getByText('Rules agents follow')).toBeVisible();
    const toggle = screen.getByRole('switch', { name: 'Enabled' });
    const before = toggle.getAttribute('aria-checked');
    await userEvent.click(toggle);
    expect(screen.getByRole('switch', { name: 'Enabled' })).not.toHaveAttribute(
      'aria-checked',
      before ?? ''
    );
  });
});
