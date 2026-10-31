import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { RENDER_DEFAULTS } from '@/state/renders';
import type { InspectView } from '@/ipc/client';
import { useInspectorStore } from '@/state/use-inspector-store';
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

  it('takes a camera stop as the camera, and leaves the option off with none', async () => {
    useRenderTabStore.setState({ style: 'clay', settings: RENDER_DEFAULTS, preview: null });
    const { unmount } = render(<RenderStage />);
    expect(screen.getByRole('button', { name: 'Camera stop' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    unmount();
    const stop = (name: string, position: [number, number, number]) => ({
      name,
      position,
      target: [0, 0, 0] as [number, number, number],
      fov: 35,
      poster: null,
    });
    useInspectorStore.setState({
      view: {
        stops: { stops: [stop('Porch', [0, 0, 5]), stop('Moon', [5, 5, 0])] },
      } as unknown as InspectView,
    });
    render(<RenderStage />);
    await userEvent.click(screen.getByRole('button', { name: 'Camera stop' }));
    expect(useRenderTabStore.getState().settings.stop).toEqual({
      name: 'Porch',
      azimuth: 0,
      elevation: 0,
    });
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Camera stop' }), 'Moon');
    expect(useRenderTabStore.getState().settings.stop).toEqual({
      name: 'Moon',
      azimuth: 90,
      elevation: 45,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Side' }));
    expect(useRenderTabStore.getState().settings).toMatchObject({ camera: 'side', stop: null });
    useInspectorStore.setState({ view: null });
  });
});
