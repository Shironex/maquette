import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import { ClipDetail } from '.';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

const item = (name: string) => {
  const clip = animationClips().find(c => c.name === name)!;
  return { clip, file: `/lib/${clip.id}/clip.glb` };
};

describe('the clip detail', () => {
  it('gives the numbers and the license note for a Mixamo clip', () => {
    render(<ClipDetail item={item('Standing Dodge Left')} />);
    const side = screen.getByRole('complementary', { name: 'Selected clip' });
    expect(within(side).getByRole('heading', { name: 'Standing Dodge Left' })).toBeVisible();
    expect(side).toHaveTextContent('Singles · from Standing Dodge Left.fbx');
    expect(side).toHaveTextContent('38 f · 1.27 s at 30 fps');
    expect(side).toHaveTextContent('yes, 0.92 m to the left');
    expect(side).toHaveTextContent('Mixamo · 22 bones');
    expect(side).toHaveTextContent('46 KB, no mesh');
    expect(side).toHaveTextContent('not to share as a raw file');
  });

  it('has no license note for a custom clip', () => {
    render(<ClipDetail item={item('Heroine walk, take 2')} />);
    expect(screen.queryByText('Mixamo clip.')).toBeNull();
    expect(screen.getByText('Custom · 22 bones')).toBeVisible();
  });
});
