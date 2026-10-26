import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import LibraryStats from './LibraryStats';

describe('the library stats', () => {
  it('shows the models, the credits reuse saved and the updates waiting', () => {
    render(<LibraryStats stats={{ models: 8, shared: 2, saved: 70, updates: 1 }} />);
    expect(screen.getByText('2 shared')).toBeVisible();
    expect(screen.getByText('70')).toBeVisible();
    expect(screen.getByText('copy behind')).toBeVisible();
  });
});
