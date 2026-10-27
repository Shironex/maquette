import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useShipStore } from '@/state/use-ship-store';
import { clearMocks, openMockShip } from '@/test/mock-ipc';
import { SiteShipView } from '.';
import { changeWords } from './SiteShipView.hooks';

beforeEach(async () => {
  await openMockShip();
  await act(() => usePipelineStore.getState().load('frostveil'));
});

afterEach(() => {
  clearMocks();
  useShipStore.setState({ view: null, error: null });
  usePipelineStore.setState({ view: null });
});

const row = (file: string) => screen.getByTitle(file).closest('tr')!;

describe('the website Ship view', () => {
  it('lists what Ship writes and holds the hand made site file back', async () => {
    render(<SiteShipView project="frostveil" onClose={vi.fn()} />);
    expect(
      await screen.findByRole('heading', { level: 1, name: "Ship to Frostveil's site" })
    ).toBeInTheDocument();
    expect(screen.getByText(/3 written · 1 held back · 0 removed/)).toBeInTheDocument();
    expect(
      within(row('observatory_arch.glb')).getByText('Held: the site file was not made by Maquette')
    ).toBeInTheDocument();
    expect(within(row('snow_drift.glb')).getByText('Left alone')).toBeInTheDocument();
  });

  it('ships a hand made file once the person picks Always', async () => {
    render(<SiteShipView project="frostveil" onClose={vi.fn()} />);
    await screen.findByTitle('observatory_arch.glb');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Ship observatory_arch.glb' }),
      'always'
    );
    expect(await screen.findByText(/4 written · 0 held back/)).toBeInTheDocument();
  });

  it('switches to a hand-built site and holds a smaller site file', async () => {
    render(<SiteShipView project="frostveil" onClose={vi.fn()} />);
    await screen.findByTitle('crystal_lantern.glb');
    await userEvent.click(screen.getByRole('button', { name: 'Hand-built site' }));
    expect(
      await within(row('crystal_lantern.glb')).findByText('Held: the site file is smaller')
    ).toBeInTheDocument();
    expect(useShipStore.getState().view?.settings.style).toBe('models');
  });

  it('renames a model and keeps the name', async () => {
    render(<SiteShipView project="frostveil" onClose={vi.fn()} />);
    const field = await screen.findByRole('textbox', { name: 'File name of Veil platform' });
    await userEvent.type(field, 'veil{Enter}');
    expect(await screen.findByTitle('veil.glb')).toBeInTheDocument();
    expect(useShipStore.getState().view?.settings.assets['veil-platform']).toEqual({
      choice: 'auto',
      file: 'veil',
    });
  });

  it('says what each change does to the site file', () => {
    expect(changeWords('new', null)).toBe('new to the site');
    expect(changeWords('replaced', '1.6 MB')).toBe('replaces 1.6 MB');
    expect(changeWords('unchanged', null)).toBe('same bytes, nothing to do');
  });
});
