import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Panel } from '.';

describe('the panel', () => {
  it('names its region after the title and shows the help and actions', () => {
    render(
      <Panel
        id="budgets"
        title="Budgets"
        help="Over budget shows in red"
        actions={<button type="button">Reset</button>}
      >
        <p>Scene target</p>
      </Panel>
    );
    const region = screen.getByRole('region', { name: 'Budgets' });
    expect(region).toHaveAttribute('id', 'budgets');
    expect(region).toHaveTextContent('Over budget shows in red');
    expect(region).toHaveTextContent('Scene target');
    expect(screen.getByRole('button', { name: 'Reset' })).toBeVisible();
  });

  it('renders only its children without a title', () => {
    render(<Panel>Plain body</Panel>);
    expect(screen.queryByRole('heading')).toBeNull();
    expect(screen.getByText('Plain body')).toBeVisible();
  });
});
