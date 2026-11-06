import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import RecipePreview, { ROUGH_LABEL } from './RecipePreview';

const base = {
  project: 'p',
  layers: [],
  markers: [],
  share: 0,
  playing: false,
  onTick: () => undefined,
};

describe('the recipe preview', () => {
  it('says it is rough, and asks for a model until one is picked', () => {
    render(<RecipePreview {...base} model={null} clip={null} />);
    expect(screen.getByText(ROUGH_LABEL)).toBeVisible();
    expect(screen.getByText(/Pick a rigged model/)).toBeVisible();
  });

  it('says when the base clip is not in the library', () => {
    render(
      <RecipePreview
        {...base}
        model={{ asset: 'clay', name: 'Clay figure', file: '/m.glb' }}
        clip={null}
      />
    );
    expect(screen.getByText(/not in the animation library/)).toBeVisible();
  });

  it('needs no base clip on the rest pose', () => {
    render(
      <RecipePreview
        {...base}
        project={null}
        model={{ asset: 'clay', name: 'Clay figure', file: '/m.glb' }}
        clip="rest"
      />
    );
    expect(screen.queryByText(/not in the animation library/)).not.toBeInTheDocument();
    expect(screen.getByText(ROUGH_LABEL)).toBeVisible();
  });
});
