import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import { ClipPreview } from '.';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

describe('the clip preview', () => {
  it('starts at the first frame and says how far the root travels', () => {
    const clip = animationClips().find(c => c.name === 'Run To Rolling')!;
    render(<ClipPreview item={{ clip, file: '/lib/run/clip.glb', uses: [] }} />);
    expect(screen.getByText('frame 0 / 52')).toBeVisible();
    expect(screen.getByText('2.26 m forward')).toBeVisible();
    expect(screen.getByText('0.00 s')).toBeVisible();
  });
});
