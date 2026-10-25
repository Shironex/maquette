import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useOnboarding } from '../../Onboarding/Onboarding.hooks';
import { DefaultsStep } from '.';

afterEach(() => clearMocks());

function Harness() {
  const api = useOnboarding();
  return (
    <>
      <output aria-label="step">{api.step}</output>
      <DefaultsStep api={api} />
    </>
  );
}

describe('the defaults step', () => {
  it('groups the presets, budgets and compression', async () => {
    await openMockWorkspace('first-run');
    render(<Harness />);
    expect(screen.getByRole('heading', { name: '3D provider presets' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Web budgets' })).toBeVisible();
    expect(screen.getByRole('heading', { name: 'Compression' })).toBeVisible();
    const field = screen.getByRole('textbox', { name: 'Scene target' });
    await userEvent.clear(field);
    await userEvent.type(field, '5{Enter}');
    expect(screen.getByRole('textbox', { name: 'Scene target' })).toHaveValue('5.0');
  });
});
