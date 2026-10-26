import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import MotionStage from './MotionStage';
import { Scene, type SceneProps } from './Scene';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

describe('the motion stage', () => {
  it('is one labelled canvas holding the scene with every prop but the class and label', () => {
    const props = {
      url: 'mock://figure.glb',
      clip: 1,
      time: 0.5,
      skeleton: true,
      trail: false,
      turntable: false,
      poses: null,
      outline: [],
    };
    render(<MotionStage {...props} label="Clay figure playing Walk loop" className="size-full" />);
    expect(screen.getByLabelText('Clay figure playing Walk loop')).toHaveClass('size-full');
    const scene = fiber.canvas?.children as ReactElement<SceneProps>;
    expect(isValidElement(scene) && scene.type).toBe(Scene);
    expect(scene.props).toEqual(props);
  });
});
