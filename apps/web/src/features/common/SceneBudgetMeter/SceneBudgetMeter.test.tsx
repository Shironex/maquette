import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { SceneBudgetMeter } from '.';

afterEach(() => usePipelineStore.setState({ view: null }));

describe('the scene budget meter', () => {
  it('says how much of the budget the scene uses, before the first view is measured', async () => {
    render(<SceneBudgetMeter value={2.345} target={3} />);
    await userEvent.tab();
    expect(screen.getByRole('tooltip')).toHaveTextContent(
      'The whole scene: 2.3 of 3 MB. The first view is measured once the layout loads.'
    );
    expect(screen.getByText('Scene budget')).toBeVisible();
  });

  it('carries a short label for a narrow top bar, and keeps the meter named in full', () => {
    render(<SceneBudgetMeter value={2.345} target={3} />);
    expect(screen.getByText('Scene')).toBeInTheDocument();
    expect(screen.getByRole('meter', { name: /^Scene budget, / })).toBeInTheDocument();
  });
});
