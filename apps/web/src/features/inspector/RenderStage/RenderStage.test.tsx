import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { RENDER_DEFAULTS } from '@/state/renders';
import { useRenderTabStore } from '@/state/use-render-tab-store';
import RenderStage from './RenderStage';

const shot = (style: 'clay' | 'toon', turn: number, size: number) => ({
  style,
  turn,
  size,
  file: `/x/${style}-${turn}.png`,
  seconds: 1.4,
  frames: [],
  moving: null,
  estimateBytes: null,
});

afterEach(() => useRenderTabStore.setState({ preview: null, style: 'blueprint' }));

describe('the render stage', () => {
  it('says how the picture was made and picks a style from the strip', async () => {
    useRenderTabStore.setState({
      style: 'clay',
      settings: RENDER_DEFAULTS,
      preview: {
        key: 'a',
        name: 'Street lamp',
        version: 2,
        heightM: 1.84,
        hasColour: true,
        folder: 'street-lamp',
        dir: '/x',
        shots: [shot('clay', 0, 520), shot('toon', 0, 160), shot('clay', 90, 160)],
      },
    });
    render(<RenderStage />);
    expect(screen.getByText('Preview · 520 px · rendered in 1.4 s')).toBeVisible();
    expect(screen.getByText('STREET LAMP · 1.84 m · v2')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'toon' }));
    expect(useRenderTabStore.getState().style).toBe('toon');
  });
});
