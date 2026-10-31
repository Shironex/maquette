import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import RoughStage from './RoughStage';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

describe('the rough stage', () => {
  it('draws one labelled canvas', () => {
    render(<RoughStage modelUrl="/m.glb" clipUrl="/c.glb" pairs={[]} label="Dodge on Clay" />);
    expect(screen.getByRole('img', { name: 'Dodge on Clay' })).toBeInTheDocument();
  });
});
