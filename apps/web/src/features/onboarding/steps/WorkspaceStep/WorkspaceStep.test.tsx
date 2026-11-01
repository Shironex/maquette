import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useOnboarding } from '../../Onboarding/Onboarding.hooks';
import { WorkspaceStep } from '.';

afterEach(() => clearMocks());

function Harness() {
  const api = useOnboarding();
  return (
    <>
      <output aria-label="step">{api.step}</output>
      <WorkspaceStep api={api} />
    </>
  );
}

describe('the workspace step', () => {
  it('shows the folders it will create and checks a typed root', async () => {
    await openMockWorkspace('first-run');
    render(<Harness />);
    expect(screen.getByText('projects/')).toBeVisible();
    expect(screen.getByText(/Finished 3D models land here/)).toBeVisible();
    const field = screen.getByRole('textbox', { name: 'Root folder' });
    await userEvent.clear(field);
    await userEvent.type(field, '~/Documents{Enter}');
    await waitFor(() => expect(screen.getByText(/This folder has other files/)).toBeVisible());
  });
});
