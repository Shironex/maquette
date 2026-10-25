import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProjectManifest } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { mockAsset } from '../fakeBoardEnv';
import RenameAssetDialog from './RenameAssetDialog';

beforeEach(async () => {
  await openMockWorkspace();
});

afterEach(() => clearMocks());

function renderDialog() {
  const onDone = vi.fn<(project: ProjectManifest) => void>();
  const onClose = vi.fn();
  render(
    <RenameAssetDialog
      project="moon-island"
      asset={mockAsset('Island')}
      onDone={onDone}
      onClose={onClose}
    />
  );
  return { onDone, onClose, field: screen.getByRole('textbox', { name: 'Name' }) };
}

describe('RenameAssetDialog', () => {
  it('renames the asset', async () => {
    const { onDone, field } = renderDialog();
    expect(field).toHaveValue('Island');
    await userEvent.clear(field);
    await userEvent.type(field, 'Floating island');
    await userEvent.click(screen.getByRole('button', { name: 'Rename' }));
    await vi.waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    expect(onDone.mock.calls[0]?.[0].assets?.map(a => a.name)).toContain('Floating island');
  });

  it('closes without a call when the name did not change', async () => {
    const { onDone, onClose } = renderDialog();
    await userEvent.click(screen.getByRole('button', { name: 'Rename' }));
    expect(onClose).toHaveBeenCalledOnce();
    expect(onDone).not.toHaveBeenCalled();
  });

  it('holds a name over the limit', async () => {
    const { field } = renderDialog();
    await userEvent.clear(field);
    await userEvent.type(field, 'x'.repeat(81));
    expect(screen.getByText('Keep the name to 80 characters.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Rename' })).toBeDisabled();
  });
});
