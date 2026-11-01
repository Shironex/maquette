import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ShipTarget, SiteShipPlan } from '@/ipc/client';
import { ShipSettingsPanel } from '.';
import { folderBlock, targetOf } from './ShipSettingsPanel.hooks';

const target: ShipTarget = {
  repo: '/sites/room',
  models: 'public/models',
  posters: 'public/posters',
  data: null,
  configured: true,
  modelsExist: true,
  error: null,
};

const plan = (patch: Partial<ShipTarget> = {}): SiteShipPlan => ({
  target: { ...target, ...patch },
  style: 'scene',
  textures: 'ktx2',
  exported: true,
  files: [],
  left: [],
  checks: [{ ok: true, text: 'A site repo is set' }],
  scriptReady: true,
  originReady: true,
});

const settings = { version: 1, style: 'scene', textures: 'ktx2', quality: 80, assets: {} } as const;

describe('the Ship settings panel', () => {
  it('saves the style and the texture format', async () => {
    const onSettings = vi.fn().mockResolvedValue(true);
    render(
      <ShipSettingsPanel
        settings={settings}
        plan={plan()}
        repo="~/sites/room"
        saving={false}
        onSettings={onSettings}
        onTarget={vi.fn()}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: 'Hand-built site' }));
    await userEvent.click(screen.getByRole('button', { name: 'WebP' }));
    expect(onSettings).toHaveBeenCalledWith({ style: 'models' });
    expect(onSettings).toHaveBeenCalledWith({ textures: 'webp' });
    expect(screen.getByRole('textbox', { name: 'Image quality' })).toBeDisabled();
  });

  it('writes the folders, with empty posters as none', async () => {
    const onTarget = vi.fn().mockResolvedValue(true);
    render(
      <ShipSettingsPanel
        settings={settings}
        plan={plan()}
        repo="~/sites/room"
        saving={false}
        onSettings={vi.fn()}
        onTarget={onTarget}
      />
    );
    const models = screen.getByRole('textbox', { name: 'Models folder' });
    await userEvent.clear(models);
    await userEvent.type(models, 'apps/landing/public/room/models/');
    await userEvent.clear(screen.getByRole('textbox', { name: 'Posters folder' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save folders' }));
    expect(onTarget).toHaveBeenCalledWith({
      models: 'apps/landing/public/room/models',
      posters: null,
      data: null,
    });
  });

  it('says why the folders cannot be saved', () => {
    const same = { models: 'public/models', posters: 'public/posters', data: '' };
    expect(folderBlock(same, target)).toBe('Nothing changed yet');
    expect(folderBlock(same, { ...target, repo: null })).toBe('Waits for a site repo');
    expect(folderBlock({ ...same, models: ' ' }, target)).toBe('Name the models folder');
    expect(targetOf({ models: 'a/', posters: '', data: 'src/x.json' })).toEqual({
      models: 'a',
      posters: null,
      data: 'src/x.json',
    });
  });
});
