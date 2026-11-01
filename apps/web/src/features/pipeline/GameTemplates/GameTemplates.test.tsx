import { act, render, screen, waitFor, within } from '@testing-library/react';
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
  exportBefore: { textures: 'png', textureCap: 1024, dropMaps: false },
  exportAfter: { textures: 'png', textureCap: 1024, dropMaps: false },
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

/** A preview call the test answers itself. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(r => (resolve = r));
  return { promise, resolve };
}

describe('GameTemplates', () => {
  beforeEach(() => {
    previewTemplate.mockReset().mockResolvedValue(preview);
    applyTemplate.mockReset().mockResolvedValue(true);
    useGameStore.setState({ previewTemplate, applyTemplate, saving: false });
  });

  it('says it is reading while a preview waits, and a late answer cannot replace a newer pick', async () => {
    const first = deferred<GameTemplatePreview | null>();
    const second = deferred<GameTemplatePreview | null>();
    previewTemplate.mockReset();
    previewTemplate.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const sheet = await openSheet();
    const cards = await within(sheet).findAllByRole('button', { pressed: false });

    await userEvent.click(cards[0]!);
    expect(within(sheet).getByRole('status')).toHaveTextContent(/^Reading this game/);
    await userEvent.click(cards[1]!);

    second.resolve(null);
    await waitFor(() =>
      expect(within(sheet).getByRole('status')).toHaveTextContent(/^Could not read this game/)
    );
    await act(async () => first.resolve(preview));
    expect(within(sheet).queryByText('What will change')).toBeNull();
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
    expect(
      within(within(sheet).getByRole('region', { name: 'What will change' })).getByText('.obj')
    ).toBeInTheDocument();
    expect(within(sheet).getByText('Models over budget: 3 now, 1 after.')).toBeInTheDocument();
    expect(within(sheet).getByText(/are not \.obj/)).toBeInTheDocument();
    expect(within(sheet).getByText('props/lantern.glb')).toBeInTheDocument();
    expect(within(sheet).getByText('wall.glb to kit/wall.obj')).toBeInTheDocument();
    expect(within(sheet).getByText(/No file moves or goes away/)).toBeInTheDocument();

    await userEvent.click(within(sheet).getByRole('button', { name: 'Apply template' }));
    expect(applyTemplate).toHaveBeenCalledWith('crypt', 'level-kit-obj');
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('groups the templates and puts the numbers on each card', async () => {
    const sheet = await openSheet();
    const groups = await within(sheet).findAllByRole('group');
    expect(groups.map(g => g.getAttribute('aria-label'))).toEqual([
      'By shape of the models',
      'By engine',
      'By device or kind of project',
    ]);
    const unity = within(sheet).getByRole('button', { name: /^Unity/ });
    expect(unity).toHaveTextContent('.glb, PNG up to 2048 px');
    expect(unity).toHaveTextContent('props/ 15k, characters/ 40k, kit/ 5k');
    expect(within(sheet).getByRole('button', { name: /^Game jam/ })).toHaveTextContent(
      'all folders 20k'
    );
  });

  it('lists the texture settings a template changes next to the budgets', async () => {
    previewTemplate.mockResolvedValue({
      ...preview,
      template: GAME_TEMPLATES.find(t => t.id === 'game-jam')!,
      formatAfter: 'glb',
      exportAfter: { textures: 'jpg', textureCap: 512, dropMaps: true },
    });
    const sheet = await openSheet();
    await userEvent.click(await within(sheet).findByRole('button', { name: /^Game jam/ }));
    await within(sheet).findByText('Texture format');
    expect(within(sheet).getByText('Texture size cap')).toBeInTheDocument();
    expect(within(sheet).getByText('512 px')).toBeInTheDocument();
    expect(within(sheet).getByText('Normal and metal maps')).toBeInTheDocument();
    expect(within(sheet).getByText('left out')).toBeInTheDocument();
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
