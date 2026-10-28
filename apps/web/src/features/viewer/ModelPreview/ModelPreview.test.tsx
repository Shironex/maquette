import { render, screen } from '@testing-library/react';
import { isValidElement, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fiber, resetFiber } from '@/test/fake-fiber';
import ModelPreview from './ModelPreview';
import { Studio, type StudioProps } from './Studio';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

beforeEach(() => {
  resetFiber();
});

function studio(): StudioProps {
  const children = fiber.canvas?.children as ReactElement[];
  const found = children.find(c => isValidElement(c) && c.type === Studio);
  return (found as ReactElement<StudioProps>).props;
}

describe('the model preview', () => {
  it('draws one canvas, shaded and turning by default', () => {
    render(<ModelPreview url="/mock/model.glb" className="absolute! inset-0" />);
    const canvas = screen.getByRole('img', { name: 'Model preview' });
    expect(canvas).toHaveClass('absolute!', 'inset-0');
    expect(studio()).toMatchObject({
      source: { url: '/mock/model.glb' },
      mode: 'shaded',
      turntable: true,
    });
  });

  it('hands the studio the chosen look and the callbacks for the file', () => {
    const onLoaded = vi.fn();
    const onError = vi.fn();
    render(
      <ModelPreview
        url="/mock/rig.glb"
        mode="bones"
        turntable={false}
        onLoaded={onLoaded}
        onError={onError}
      />
    );
    expect(studio()).toEqual({
      source: { url: '/mock/rig.glb', onLoaded, onError },
      mode: 'bones',
      turntable: false,
    });
  });
});
