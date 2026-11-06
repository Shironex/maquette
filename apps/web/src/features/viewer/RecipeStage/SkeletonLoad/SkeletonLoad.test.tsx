import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import SkeletonLoad from './SkeletonLoad';

const loaded = { scene: {}, originals: new Map(), bones: [], clips: [] };

vi.mock('../../MotionStage/Scene/Scene.hooks', () => ({
  useMotionModel: (url: string | null) => (url ? loaded : null),
}));

describe('loading one more skeleton', () => {
  it('says when the skeleton is in and when it goes', () => {
    const onLoad = vi.fn();
    const view = render(<SkeletonLoad url="/clips/block.glb" onLoad={onLoad} />);
    expect(onLoad).toHaveBeenLastCalledWith('/clips/block.glb', loaded);
    view.unmount();
    expect(onLoad).toHaveBeenLastCalledWith('/clips/block.glb', null);
  });
});
