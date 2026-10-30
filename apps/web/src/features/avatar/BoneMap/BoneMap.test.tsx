import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openAvatar } from '@/test/open-avatar';
import BoneMap from './BoneMap';

afterEach(() => clearMocks());

describe('the bone map', () => {
  it('shows the required bones and the rest on asking', async () => {
    await openAvatar();
    render(<BoneMap />);
    expect(screen.getByText('Required · 15 of 15')).toBeInTheDocument();
    expect(screen.getByText('mixamorig:LeftArm')).toBeInTheDocument();
    expect(screen.queryByText('Left eye')).not.toBeInTheDocument();
    expect(screen.getByText('Optional body · 7 of 10')).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole('button', { name: 'Show' })[0]!);
    expect(screen.getByText('Left eye').closest('li')).toHaveTextContent('missing');
  });
});
