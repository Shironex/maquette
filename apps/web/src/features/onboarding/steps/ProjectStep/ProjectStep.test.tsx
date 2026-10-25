import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useOnboarding } from '../../Onboarding/Onboarding.hooks';
import { ProjectStep } from '.';

afterEach(() => clearMocks());

function Harness() {
  const api = useOnboarding();
  return (
    <>
      <output aria-label="step">{api.step}</output>
      <ProjectStep api={api} />
    </>
  );
}

describe('the first project step', () => {
  it('edits the draft project name', async () => {
    await openMockWorkspace('first-run');
    render(<Harness />);
    const name = screen.getByLabelText('Project name');
    expect(name).toHaveValue('Moon Island');
    await userEvent.clear(name);
    await userEvent.type(name, 'Harbour lights');
    expect(screen.getByLabelText('Project name')).toHaveValue('Harbour lights');
  });
});
