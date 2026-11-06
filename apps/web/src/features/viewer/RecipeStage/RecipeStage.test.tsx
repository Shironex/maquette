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
        play={{ share: 0, playing: false, onTick: () => undefined }}
        label="Reach on Clay, rough preview"
      />
    );
    expect(screen.getByRole('img', { name: 'Reach on Clay, rough preview' })).toBeInTheDocument();
  });

  it('draws the rest pose with no clip file', () => {
    render(
      <RecipeStage
        modelUrl="/m.glb"
        clipUrl={null}
        pairs={[]}
        layers={[]}
        markers={[]}
        play={{ share: 0, playing: false, onTick: () => undefined }}
        label="The rest pose on Clay, rough preview"
      />
    );
    expect(
      screen.getByRole('img', { name: 'The rest pose on Clay, rough preview' })
    ).toBeInTheDocument();
  });
});
