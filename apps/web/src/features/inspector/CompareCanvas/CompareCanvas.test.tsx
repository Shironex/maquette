import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import { Compare, type CompareProps } from './Compare';
import CompareCanvas from './CompareCanvas';
import type { CompareCanvasProps } from './CompareCanvas.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

const props: CompareCanvasProps = {
  kind: 'asset',
  left: [{ id: 'crate-v1', url: 'mock://v1', position: [0, 0, 0] }],
  right: [{ id: 'crate-v2', url: 'mock://v2', position: [0, 0, 0] }],
  split: 0.5,
  mode: 'wireframe',
  turntable: false,
  stops: [],
  stop: null,
  handlers: {
    onSample: vi.fn(),
    onPick: vi.fn(),
    onStopShots: vi.fn(),
    onUserMove: vi.fn(),
    onLightmaps: vi.fn(),
    onError: vi.fn(),
    onReady: vi.fn(),
  },
};

describe('the compare canvas', () => {
  it('is one labelled canvas holding the compare view with every prop but the class', () => {
    render(<CompareCanvas {...props} className="absolute! inset-0" />);
    expect(screen.getByLabelText('Compare view')).toHaveClass('absolute!', 'inset-0');
    const compare = fiber.canvas?.children as ReactElement<CompareProps>;
    expect(isValidElement(compare) && compare.type).toBe(Compare);
    expect(compare.props).toEqual(props);
  });
});
