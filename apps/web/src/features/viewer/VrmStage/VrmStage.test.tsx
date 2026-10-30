import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import VrmStage from './VrmStage';
import { Scene, type SceneProps } from './Scene';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

describe('the vrm stage', () => {
  it('is one labelled canvas holding the scene with every prop but the class and label', () => {
    const props = { url: 'mock://hero.vrm', armsDown: true, breathe: false, lookAt: true };
    render(<VrmStage {...props} label="Hero as a VRM" className="size-full" />);
    expect(screen.getByLabelText('Hero as a VRM')).toHaveClass('size-full');
    const scene = fiber.canvas?.children as ReactElement<SceneProps>;
    expect(isValidElement(scene) && scene.type).toBe(Scene);
    expect(scene.props).toEqual(props);
  });
});
