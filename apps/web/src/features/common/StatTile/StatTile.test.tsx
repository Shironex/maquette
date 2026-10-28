import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { StatTile } from '.';

describe('the stat tile', () => {
  it('shows the label, the value with its note and the tone', () => {
    render(<StatTile label="On disk" value="23.6 MB" note="skeleton only" tone="text-sage" />);
    expect(screen.getByText('On disk')).toBeVisible();
    expect(screen.getByText('23.6 MB').parentElement).toHaveClass('text-sage');
    expect(screen.getByText('skeleton only')).toBeVisible();
  });
});
