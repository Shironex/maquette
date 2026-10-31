import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRetargetStore } from '@/state/use-retarget-store';
import { animationClips } from '@/ipc/mock/animation-fixtures';
import { ClipDetail } from '.';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

const item = (name: string) => {
  const clip = animationClips().find(c => c.name === name)!;
  return { clip, file: `/lib/${clip.id}/clip.glb`, uses: [] };
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

  it('lists the models that baked it and offers an update for an older take', () => {
    const open = vi.fn();
    useRetargetStore.setState({ open });
    const use = {
      project: 'frostveil',
      projectName: 'Frostveil',
      asset: 'elsheria',
      assetName: 'Elsheria',
      name: 'dodge_left',
      version: 2,
      revision: 1,
      outdated: true,
    };
    render(<ClipDetail item={{ ...item('Standing Dodge Left'), uses: [use] }} />);
    const used = screen.getByRole('region', { name: 'Where it is used' });
    expect(used).toHaveTextContent('Elsheria v2 · Frostveil · dodge_left');
    fireEvent.click(within(used).getByRole('button', { name: 'Update' }));
    expect(open).toHaveBeenCalledWith({
      project: 'frostveil',
      asset: 'elsheria',
      clips: ['standing-dodge-left'],
      step: 'settings',
    });
    expect(screen.getByRole('button', { name: 'Use on model' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('tries the clip on the first rigged model and goes back to its skeleton', () => {
    useAppStore.setState({
      workspace: { project: { id: 'frostveil' } },
    } as unknown as Partial<ReturnType<typeof useAppStore.getState>>);
    useMotionStore.setState({
      load: vi.fn(),
      view: {
        project: 'frostveil',
        models: [{ asset: 'clay', name: 'Clay figure', rig: { bones: 20 } }],
      },
    } as unknown as Partial<ReturnType<typeof useMotionStore.getState>>);
    render(<ClipDetail item={item('Standing Dodge Left')} />);
    fireEvent.click(screen.getByRole('button', { name: 'Try on Clay figure' }));
    expect(screen.getByRole('button', { name: 'Back to its skeleton' })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Back to its skeleton' }));
    expect(screen.getByRole('button', { name: 'Try on Clay figure' })).toBeVisible();
  });
});
