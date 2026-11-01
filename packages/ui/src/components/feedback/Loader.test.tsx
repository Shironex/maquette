import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BudgetMeter } from '../status/BudgetMeter';
import { Loader, WaitLine } from './Loader';

describe('the waiting recipe', () => {
  it('says the verb alone when nothing else is known', () => {
    render(<Loader label="Reading the motion..." />);
    expect(screen.getByRole('status')).toHaveTextContent(/^Reading the motion\.\.\.$/);
  });

  it('adds the count and the past time when they are known', () => {
    render(<Loader label="Loading the models..." count="3 of 13" estimate="last 12s" />);
    expect(screen.getByText('3 of 13 · last 12s')).toBeInTheDocument();
  });

  it('reads the same on one line', () => {
    render(<WaitLine label="Rendering..." estimate="last 9s" />);
    expect(screen.getByRole('status')).toHaveTextContent('Rendering...last 9s');
  });
});

describe('a budget meter before its number is known', () => {
  it('says so instead of showing a zero', () => {
    render(<BudgetMeter label="First view" value={null} target={3} unit="MB" />);
    expect(screen.getByText('Reading...')).toBeInTheDocument();
    expect(screen.queryByText('0.0')).toBeNull();
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuetext', 'Reading...');
  });
});
