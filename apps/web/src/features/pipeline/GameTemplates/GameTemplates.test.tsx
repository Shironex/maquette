import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameTemplatePreview } from '@/ipc/client';
import { GAME_TEMPLATES } from '@/ipc/mock/game-templates';
import { useGameStore } from '@/state/use-game-store';
import { GameTemplates } from '.';

vi.mock('@/ipc/client', async importOriginal => ({
  ...(await importOriginal<typeof import('@/ipc/client')>()),
  commands: { gameTemplateList: () => Promise.resolve(GAME_TEMPLATES) },
}));

const kit = GAME_TEMPLATES.find(t => t.id === 'level-kit-obj')!;
const preview: GameTemplatePreview = {
  template: kit,
  formatBefore: 'glb',
  formatAfter: 'obj',
  budgets: [
    {
      folder: 'kit',
      before: null,
      after: { folder: 'kit', triangles: 5_000, texture: 1024 },
    },
  ],
  keptFiles: ['props/lantern.glb'],
  newPaths: [{ asset: 'wall', from: 'wall.glb', to: 'kit/wall.obj' }],
  overBefore: 3,
  overAfter: 1,
  notes: [
    'Most of the 16 models in the model folder are not .obj. Check that the engine reads it.',
  ],
  changes: true,
};

const previewTemplate = vi.fn();
const applyTemplate = vi.fn();

async function openSheet(busy = false) {
  render(<GameTemplates project="crypt" busy={busy} />);
  await userEvent.click(screen.getByRole('button', { name: 'Use a template' }));
  return screen.getByRole('dialog', { name: 'Use a template' });
}

describe('GameTemplates', () => {
  beforeEach(() => {
    previewTemplate.mockReset().mockResolvedValue(preview);
    applyTemplate.mockReset().mockResolvedValue(true);
    useGameStore.setState({ previewTemplate, applyTemplate, saving: false });
  });

  it('shows what will change before anything is saved, then applies', async () => {
    const sheet = await openSheet();
    expect(within(sheet).getByRole('button', { name: 'Apply template' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await userEvent.click(await within(sheet).findByRole('button', { name: /^Level kit, \.obj/ }));
    await within(sheet).findByText('What will change');
    expect(previewTemplate).toHaveBeenCalledWith('crypt', 'level-kit-obj');
    expect(applyTemplate).not.toHaveBeenCalled();
    expect(within(sheet).getByText('no budget')).toBeInTheDocument();
    expect(within(sheet).getByText('5k triangles, 1024 px')).toBeInTheDocument();
    expect(within(sheet).getByText('.obj')).toBeInTheDocument();
    expect(within(sheet).getByText('Models over budget: 3 now, 1 after.')).toBeInTheDocument();
    expect(within(sheet).getByText(/are not \.obj/)).toBeInTheDocument();
    expect(within(sheet).getByText('props/lantern.glb')).toBeInTheDocument();
    expect(within(sheet).getByText('wall.glb to kit/wall.obj')).toBeInTheDocument();
    expect(within(sheet).getByText(/No file moves or goes away/)).toBeInTheDocument();

    await userEvent.click(within(sheet).getByRole('button', { name: 'Apply template' }));
    expect(applyTemplate).toHaveBeenCalledWith('crypt', 'level-kit-obj');
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('waits for a running pipeline before it applies', async () => {
    const sheet = await openSheet(true);
    await userEvent.click(await within(sheet).findByRole('button', { name: /^Level kit, \.obj/ }));
    await userEvent.click(await within(sheet).findByRole('button', { name: 'Apply template' }));
    expect(applyTemplate).not.toHaveBeenCalled();
  });

  it('has nothing to apply when the game already matches', async () => {
    previewTemplate.mockResolvedValue({ ...preview, budgets: [], changes: false });
    const sheet = await openSheet();
    await userEvent.click(await within(sheet).findByRole('button', { name: /^Prop set/ }));
    await within(sheet).findByText('This game already has these settings.');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Apply template' }));
    expect(applyTemplate).not.toHaveBeenCalled();
  });

  it('stays open when the apply fails', async () => {
    applyTemplate.mockResolvedValue(false);
    const sheet = await openSheet();
    await userEvent.click(await within(sheet).findByRole('button', { name: /^Level kit, \.obj/ }));
    await userEvent.click(await within(sheet).findByRole('button', { name: 'Apply template' }));
    expect(applyTemplate).toHaveBeenCalled();
    expect(screen.getByRole('dialog', { name: 'Use a template' })).toBeInTheDocument();
  });
});
