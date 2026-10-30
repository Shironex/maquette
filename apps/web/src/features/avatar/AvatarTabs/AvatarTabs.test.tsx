import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import AvatarTabs from './AvatarTabs';

afterEach(() => clearMocks());

describe('the avatar tabs', () => {
  it('build Export only and show the rest as later', async () => {
    await openAvatar();
    render(<AvatarTabs />);
    expect(screen.getByRole('tab', { name: 'Export' })).toHaveAttribute('aria-selected', 'true');
    for (const later of ['Springs', 'Expressions', 'Preview']) {
      expect(screen.getByText(later)).toHaveAttribute('title', expect.stringMatching(/planned/));
    }
  });
});
