import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useRenderStore } from '@/state/use-render-store';
import { useRenderTabStore } from '@/state/use-render-tab-store';
import { RENDER_DEFAULTS } from '@/state/renders';
import { clearMocks } from '@/test/mock-ipc';
import { openMockProject } from '@/test/openMockProject';
import RenderPanel from './RenderPanel';

beforeEach(() => {
  useRenderTabStore.setState({
    picked: null,
    style: 'blueprint',
    settings: RENDER_DEFAULTS,
    preview: null,
    turntableOpen: false,
  });
  useRenderStore.setState({ recent: [], live: null, record: null, open: false });
});
afterEach(() => clearMocks());

describe('the Render tab', () => {
  it('shows the blueprint of the first model with its settings and file', async () => {
    await openMockProject();
    render(<RenderPanel />);
    expect(screen.getByRole('button', { name: /^Blueprint/ })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await screen.findByText(/^Preview · 520 px/);
    expect(screen.getByText('Saved to')).toBeVisible();
    expect(screen.getByText(/blueprint\.png$/)).toBeVisible();
    expect(screen.getByLabelText('Title block')).toBeChecked();
  });

  it('keeps the blueprint settings to blueprint', async () => {
    await openMockProject();
    render(<RenderPanel />);
    await userEvent.click(screen.getByRole('button', { name: /^Clay/ }));
    expect(screen.queryByLabelText('Title block')).toBeNull();
    expect(screen.getByText(/clay\.png$/)).toBeVisible();
  });

  it('starts a render of the style on show', async () => {
    await openMockProject();
    render(<RenderPanel />);
    await userEvent.click(screen.getByRole('button', { name: /^Clay/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Render' }));
    await waitFor(async () => {
      const [record] = await commands.renderRecent();
      expect(record?.items[0]?.files.map(f => f.style)).toEqual(['clay']);
    });
  });

  it('says there is nothing to render on an asset with no model', async () => {
    await openMockProject();
    const noModel = useAppStore
      .getState()
      .workspace?.project?.assets?.find(a => a.kind !== 'scene' && a.versions.length === 0);
    useRenderTabStore.setState({ picked: noModel?.id ?? null });
    render(<RenderPanel />);
    expect(screen.getByRole('heading', { name: 'Nothing to render yet' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Pick an asset' }));
    await screen.findByRole('button', { name: /^Blueprint/ });
  });

  it('opens the turntable sheet with the WebP loop chosen', async () => {
    await openMockProject();
    render(<RenderPanel />);
    await userEvent.click(screen.getByRole('button', { name: /^Turntable/ }));
    const sheet = await screen.findByRole('dialog', { name: /^Turntable/ });
    expect(within(sheet).getByRole('button', { name: 'WebP loop' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });
});
