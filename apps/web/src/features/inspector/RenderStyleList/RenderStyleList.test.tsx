import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Asset } from '@/ipc/client';
import { useRenderTabStore } from '@/state/use-render-tab-store';
import RenderStyleList from './RenderStyleList';

const asset = (id: string, versions: unknown[]) =>
  ({ id, name: id, kind: 'prop', versions }) as unknown as Asset;

describe('the list of styles', () => {
  it('picks a style and tells a model from an asset without one', async () => {
    const onPick = vi.fn();
    const a = asset('Lantern', [{ version: 2 }]);
    render(<RenderStyleList assets={[a, asset('Angel', [])]} asset={a} onPick={onPick} />);
    await userEvent.click(screen.getByRole('button', { name: /^Toon/ }));
    expect(useRenderTabStore.getState().style).toBe('toon');
    expect(screen.getByRole('option', { name: 'Lantern · v2' })).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByLabelText('Asset'), 'Angel · no model');
    expect(onPick).toHaveBeenCalledWith('Angel');
  });
});
