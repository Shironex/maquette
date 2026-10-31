import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import RoughPreview from './RoughPreview';

describe('the rough preview', () => {
  it('asks for a clip until one is ticked', () => {
    render(
      <RoughPreview project="p" asset="a" modelName="Clay figure" modelFile={null} clip={null} />
    );
    expect(screen.getByText('Tick a clip to see it on Clay figure.')).toBeVisible();
  });
});
