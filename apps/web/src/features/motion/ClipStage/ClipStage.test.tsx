import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openMotion } from '@/test/open-motion';
import ClipStage from './ClipStage';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

afterEach(() => clearMocks());

describe('the clip stage', () => {
  it('names the model, its clip and the frame, with the timeline under it', async () => {
    await openMotion();
    render(<ClipStage />);
    expect(screen.getByRole('heading', { name: 'Clay figure' })).toBeInTheDocument();
    expect(screen.getByLabelText('Clip')).toHaveValue('Idle breathe');
    expect(screen.getByText('Idle breathe · frame 0')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Timeline' })).toBeInTheDocument();
  });

  it('offers the ways to a first clip on a rig with none', async () => {
    await openMotion('rest');
    render(<ClipStage />);
    expect(screen.getByText('No clips on Clay figure yet')).toBeInTheDocument();
    expect(screen.getByText('Rest pose')).toBeInTheDocument();
  });
});
