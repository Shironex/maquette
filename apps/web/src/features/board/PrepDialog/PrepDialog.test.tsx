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
    const asset = { ...mockAsset('Elsheria'), id: 'elsheria' };
    render(<PrepDialog project="frostveil" asset={asset} onClose={onClose} />);
    expect(await screen.findByText('Measured 0.94 × 1.7 × 0.44 m')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Suggested: 170 cm high/ }));
    expect(screen.getByText('Scaled ×1, so 0.94 × 1.7 × 0.44 m')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await vi.waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    const saved = await commands.prepareGet('frostveil');
    expect(saved.models.elsheria?.size).toEqual({ axis: 'height', value: 170, unit: 'cm' });
  });

  it('closes without saving when nothing changed', async () => {
    const onClose = vi.fn();
    const save = vi.spyOn(commands, 'prepareSave');
    const asset = { ...mockAsset('Elsheria'), id: 'elsheria' };
    render(<PrepDialog project="frostveil" asset={asset} onClose={onClose} />);
    await userEvent.click(await screen.findByRole('button', { name: 'Done' }));
    expect(onClose).toHaveBeenCalledOnce();
    expect(save).not.toHaveBeenCalled();
  });
});
