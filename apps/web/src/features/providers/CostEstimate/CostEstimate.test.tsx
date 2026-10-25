import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CostEstimate } from '.';

describe('the cost estimate', () => {
  it('reads as about so many credits', () => {
    render(<CostEstimate credits={1240} />);
    expect(screen.getByText('About 1,240 credits')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'About this estimate' })).toBeInTheDocument();
  });

  it('shows nothing for a send that spends nothing', () => {
    const { container } = render(<CostEstimate credits={null} />);
    expect(container).toBeEmptyDOMElement();
  });
});
