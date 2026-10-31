import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { RENDER_DEFAULTS } from '@/state/renders';
import { useRenderTabStore } from '@/state/use-render-tab-store';
import RenderOptions from './RenderOptions';

const source = { from: 'project', project: 'p' } as const;
afterEach(() => useRenderTabStore.setState({ settings: RENDER_DEFAULTS, style: 'blueprint' }));

describe('the settings of a style', () => {
  it('changes a blueprint setting, and Reset puts it back', async () => {
    useRenderTabStore.setState({ style: 'blueprint' });
    render(<RenderOptions source={source} assetId="a" />);
    await userEvent.click(screen.getByLabelText('Title block'));
    expect(useRenderTabStore.getState().settings.titleBlock).toBe(false);
    await userEvent.click(screen.getByRole('button', { name: 'Reset' }));
    expect(useRenderTabStore.getState().settings.titleBlock).toBe(true);
  });

  it('asks for the clay colour on clay, not the lines', () => {
    useRenderTabStore.setState({ style: 'clay' });
    render(<RenderOptions source={source} assetId="a" />);
    expect(screen.getByLabelText('Clay')).toBeInTheDocument();
    expect(screen.queryByLabelText('Lines')).toBeNull();
  });
});
