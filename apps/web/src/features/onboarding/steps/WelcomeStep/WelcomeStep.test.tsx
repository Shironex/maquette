import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';

import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useOnboarding } from '../../Onboarding/Onboarding.hooks';
import { WelcomeStep } from '.';

afterEach(() => clearMocks());

function Harness() {
  const api = useOnboarding();
  return (
    <>
      <output aria-label="step">{api.step}</output>
      <WelcomeStep api={api} />
    </>
  );
}

describe('the welcome step', () => {
  it('lists the pipeline stages and starts the setup', async () => {
    await openMockWorkspace('first-run');
    render(<Harness />);
    expect(
      screen.getByLabelText('Pipeline: Concept, Refs, 3D model, Blender, Web, Ship')
    ).toHaveTextContent('ConceptRefsTripoBlenderWebShip');
    await userEvent.click(screen.getByRole('button', { name: 'Set up' }));
    expect(screen.getByLabelText('step')).toHaveTextContent('1');
  });
});
