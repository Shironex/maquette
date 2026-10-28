import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import AssembleStage from './AssembleStage';
import type { AssembleStageProps } from './AssembleStage.types';
import { Placer } from './Placer';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

const props: AssembleStageProps = {
  parts: [],
  selected: null,
  mode: 'translate',
  snap: { move: 0.05, turn: 15 },
  onSelect: vi.fn(),
  onPlace: vi.fn(),
  onError: vi.fn(),
};

describe('the assembly stage canvas', () => {
  it('is one labelled canvas holding the placer with every prop but the class', () => {
    render(<AssembleStage {...props} className="absolute! inset-0" />);
    expect(screen.getByLabelText('Assembly')).toHaveClass('absolute!', 'inset-0');
    const placer = fiber.canvas?.children as ReactElement<AssembleStageProps>;
    expect(isValidElement(placer) && placer.type).toBe(Placer);
    expect(placer.props).toEqual(props);
  });
});
