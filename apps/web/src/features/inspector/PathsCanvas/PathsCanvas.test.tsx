import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import PathsCanvas from './PathsCanvas';
import type { PathsCanvasProps } from './PathsCanvas.types';
import { PathsScene, type PathsSceneProps } from './PathsScene';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

const props: PathsCanvasProps = {
  models: [],
  layout: { models: {}, stops: [], desk: null },
  paths: [],
  selection: null,
  tool: 'add',
  snap: true,
  frame: 1,
  handlers: {
    onSelect: vi.fn(),
    onAddPoint: vi.fn(),
    onStroke: vi.fn(),
    onDragPoint: vi.fn(),
    onDropPoint: vi.fn(),
    onError: vi.fn(),
    onReady: vi.fn(),
  },
};

describe('the paths canvas', () => {
  it('is one labelled canvas holding the scene with every prop but the class', () => {
    render(<PathsCanvas {...props} className="absolute! inset-0" />);
    expect(screen.getByRole('img', { name: 'Paths in the scene' })).toHaveClass(
      'absolute!',
      'inset-0'
    );
    const scene = fiber.canvas?.children as ReactElement<PathsSceneProps>;
    expect(isValidElement(scene) && scene.type).toBe(PathsScene);
    expect(scene.props).toEqual(props);
  });
});
