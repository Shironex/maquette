import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import { Editor, type EditorProps } from './Editor';
import LayoutCanvas from './LayoutCanvas';
import type { LayoutCanvasProps } from './LayoutCanvas.types';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

const props: LayoutCanvasProps = {
  models: [],
  layout: { models: {}, stops: [], desk: [1.2, 0.6] },
  selection: null,
  tool: 'rotate',
  snap: true,
  grid: false,
  preview: null,
  frame: 2,
  handlers: {
    onSelect: vi.fn(),
    onBeginDrag: vi.fn(),
    onDragModel: vi.fn(),
    onDragStop: vi.fn(),
    onPose: vi.fn(),
    onUserMove: vi.fn(),
    onError: vi.fn(),
    onReady: vi.fn(),
  },
};

describe('the layout canvas', () => {
  it('is one labelled canvas holding the editor with every prop but the class', () => {
    render(<LayoutCanvas {...props} className="absolute! inset-0" />);
    expect(screen.getByRole('img', { name: 'Scene layout' })).toHaveClass('absolute!', 'inset-0');
    const editor = fiber.canvas?.children as ReactElement<EditorProps>;
    expect(isValidElement(editor) && editor.type).toBe(Editor);
    expect(editor.props).toEqual(props);
  });
});
