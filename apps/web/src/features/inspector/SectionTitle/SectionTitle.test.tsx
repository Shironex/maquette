import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import SectionTitle from './SectionTitle';

describe('a section title', () => {
  it('heads the section, with its note at the far end', () => {
    render(<SectionTitle aside="3 of 8">Camera stops</SectionTitle>);
    const heading = screen.getByRole('heading', { level: 2, name: 'Camera stops' });
    expect(heading.parentElement).toHaveClass('flex', 'items-baseline', 'justify-between');
    expect(screen.getByText('3 of 8')).toHaveClass('text-meta');
  });

  it('leaves the note out when there is none and takes extra classes', () => {
    const { container } = render(<SectionTitle className="gap-2">Desk</SectionTitle>);
    expect(container.firstElementChild).toHaveClass('gap-2');
    expect(container.querySelectorAll('span')).toHaveLength(0);
  });
});
