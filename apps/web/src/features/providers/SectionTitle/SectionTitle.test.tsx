import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SectionTitle } from '.';

describe('a section title', () => {
  it('shows the heading and its aside', () => {
    render(<SectionTitle aside="2 of 4">Images</SectionTitle>);
    expect(screen.getByRole('heading', { name: 'Images' })).toBeInTheDocument();
    expect(screen.getByText('2 of 4')).toBeInTheDocument();
  });

  it('leaves the aside out when there is none', () => {
    const { container } = render(<SectionTitle>Where</SectionTitle>);
    expect(container.querySelectorAll('span')).toHaveLength(0);
  });
});
