import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { SceneBudgetMeter } from '.';

afterEach(() => usePipelineStore.setState({ view: null }));

describe('the scene budget meter', () => {
  it('says how much of the budget the scene uses, before the first view is measured', () => {
    render(<SceneBudgetMeter value={2.345} target={3} />);
    expect(screen.getByRole('tooltip')).toHaveTextContent(
      'The whole scene: 2.3 of 3 MB. The first view is measured once the layout loads.'
    );
    expect(screen.getByText('Scene budget')).toBeVisible();
  });
});
