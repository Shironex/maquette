import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { STEP } from '@/state/onboarding';
import { StepHeading } from '.';

describe('the step heading', () => {
  it('counts the step and names the project on Ready', () => {
    render(<StepHeading step={STEP.ready} name="  Moon Island " />);
    expect(screen.getByText('Step 7 of 7')).toBeVisible();
    expect(screen.getByRole('heading', { level: 1, name: 'Moon Island is ready' })).toBeVisible();
  });

  it('falls back to Your project without a name', () => {
    render(<StepHeading step={STEP.ready} name=" " />);
    expect(screen.getByRole('heading', { level: 1, name: 'Your project is ready' })).toBeVisible();
  });

  it('shows nothing for the welcome', () => {
    const { container } = render(<StepHeading step={STEP.welcome} name="Moon Island" />);
    expect(container).toBeEmptyDOMElement();
  });
});
