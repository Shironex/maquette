import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useOnboarding } from '../../Onboarding/Onboarding.hooks';
import { ReadyStep } from '.';

afterEach(() => clearMocks());

function Harness() {
  const api = useOnboarding();
  return (
    <>
      <output aria-label="step">{api.step}</output>
      <ReadyStep api={api} />
    </>
  );
}

describe('the ready step', () => {
  it('sums up each step and goes back to one to edit it', async () => {
    await openMockWorkspace('first-run');
    render(<Harness />);
    const summary = screen.getByRole('region', { name: 'Setup summary' });
    expect(summary).toHaveTextContent('Workspace');
    expect(summary).toHaveTextContent('My first project, ');
    await userEvent.click(screen.getByRole('button', { name: 'Edit defaults' }));
    expect(screen.getByLabelText('step')).toHaveTextContent('3');
  });
});
