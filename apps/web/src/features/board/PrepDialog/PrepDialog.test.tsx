import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { usePrepareStore } from '@/state/use-prepare-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { mockAsset } from '../fakeBoardEnv';
import PrepDialog from './PrepDialog';

beforeEach(async () => {
  await openMockWorkspace();
  usePrepareStore.setState({ assets: {}, failed: {}, previews: {}, saving: null });
});

afterEach(() => clearMocks());

describe('PrepDialog', () => {
  it('saves a real size for the asset and closes', async () => {
    const onClose = vi.fn();
    const asset = { ...mockAsset('Character'), id: 'character' };
    render(<PrepDialog project="moon-island" asset={asset} onClose={onClose} />);
    expect(await screen.findByText('Measured 1.36 × 2 × 0.84 m')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Suggested: 170 cm high/ }));
    expect(screen.getByText('Scaled ×0.85, so 1.16 × 1.7 × 0.71 m')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    const saved = await commands.prepareGet('moon-island');
    expect(saved.models.character?.size).toEqual({ axis: 'height', value: 170, unit: 'cm' });
  });

  it('closes without saving when nothing changed', async () => {
    const onClose = vi.fn();
    const save = vi.spyOn(commands, 'prepareSave');
    const asset = { ...mockAsset('Character'), id: 'character' };
    render(<PrepDialog project="moon-island" asset={asset} onClose={onClose} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Done' }));
    expect(onClose).toHaveBeenCalledOnce();
    expect(save).not.toHaveBeenCalled();
  });
});
