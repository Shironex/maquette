import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import RecipeStage from './RecipeStage';

vi.mock('@react-three/fiber', () => import('@/test/fake-fiber'));

describe('the recipe stage', () => {
  it('draws one labelled canvas', () => {
    render(
      <RecipeStage
        modelUrl="/m.glb"
        clipUrl="/c.glb"
        pairs={[]}
        layers={[]}
        markers={[]}
        share={0}
        playing={false}
        onTick={() => undefined}
        label="Reach on Clay, rough preview"
      />
    );
    expect(screen.getByRole('img', { name: 'Reach on Clay, rough preview' })).toBeInTheDocument();
  });
});
