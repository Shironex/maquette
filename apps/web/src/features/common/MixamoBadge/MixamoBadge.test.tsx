import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MixamoBadge } from '.';

describe('the Mixamo badge', () => {
  it('says what the mark means', () => {
    render(<MixamoBadge />);
    expect(screen.getByText('Mixamo')).toHaveAttribute(
      'title',
      expect.stringContaining('kept out of export packs')
    );
  });
});
