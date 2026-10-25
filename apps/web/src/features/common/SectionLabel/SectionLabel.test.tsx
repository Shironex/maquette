import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SectionLabel } from '.';

describe('the section label', () => {
  it('shows its text with the caption style plus the given class', () => {
    render(<SectionLabel className="mt-7">Rules agents follow</SectionLabel>);
    const label = screen.getByText('Rules agents follow');
    expect(label.tagName).toBe('P');
    expect(label).toHaveClass('text-caption', 'uppercase', 'mt-7');
  });
});
