import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { GameSettings } from '@/ipc/client';
import { GameExportPanel } from '.';
import { exportChange } from './GameExportPanel.hooks';

const settings: GameSettings = {
  format: 'glb',
  modelsDir: 'assets/models',
  export: { textures: 'png', textureCap: 1024 },
  bake: false,
  budgets: [],
  models: {},
};

describe('GameExportPanel', () => {
  it('saves the texture cap and the bake, and keeps KTX2 and LOD off', async () => {
    const onSave = vi.fn().mockResolvedValue(true);
    render(<GameExportPanel settings={settings} onDisk={2e6} busy={false} onSave={onSave} />);
    expect(screen.getByText(/KTX2 and meshopt are off for game projects/)).toBeInTheDocument();
    expect(screen.getByText('not built yet')).toBeInTheDocument();
    expect(screen.getByText(/games light at runtime/)).toBeInTheDocument();
    const apply = screen.getByRole('button', { name: 'Apply' });
    expect(apply).toHaveAttribute('aria-disabled', 'true');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Texture size cap' }),
      '512 px'
    );
    await userEvent.click(screen.getByRole('switch', { name: 'Bake light' }));
    await userEvent.click(apply);
    expect(onSave).toHaveBeenCalledWith({
      format: null,
      export: { textures: 'png', textureCap: 512 },
      bake: true,
    });
  });

  it("says each folder's budget sets its texture cap, and the cap is the fallback", () => {
    const budgets = [
      { folder: 'props', triangles: 12_000, texture: 512 },
      { folder: 'characters', triangles: 60_000, texture: 2048 },
    ];
    render(
      <GameExportPanel
        settings={{ ...settings, budgets }}
        onDisk={0}
        busy={false}
        onSave={vi.fn()}
      />
    );
    expect(
      screen.getByText(
        'Folders with a budget export at its texture size: props/ 512 px, characters/ 2048 px. ' +
          'This cap is for models in folders without one.'
      )
    ).toBeInTheDocument();
  });

  it('says every model uses the cap when no folder has a budget', () => {
    render(<GameExportPanel settings={settings} onDisk={0} busy={false} onSave={vi.fn()} />);
    expect(
      screen.getByText('Every model exports at this cap: no folder has a budget.')
    ).toBeInTheDocument();
  });

  it('sends only what changed', () => {
    expect(
      exportChange(settings, { format: 'obj', textures: 'png', textureCap: 1024, bake: false })
    ).toEqual({ format: 'obj', export: null, bake: null });
  });
});
