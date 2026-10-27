import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProjectManifest } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import AddAssetForm from './AddAssetForm';

beforeEach(async () => {
  await openMockWorkspace();
});

afterEach(() => clearMocks());

describe('AddAssetForm', () => {
  it('asks for a name before adding', async () => {
    const onDone = vi.fn();
    render(<AddAssetForm project="frostveil" onDone={onDone} onCancel={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Add asset' }));
    expect(screen.getByText('Give the asset a name.')).toBeInTheDocument();
    expect(onDone).not.toHaveBeenCalled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'S');
    expect(screen.queryByText('Give the asset a name.')).toBeNull();
  });

  it('adds the asset with its kind and column', async () => {
    const onDone = vi.fn<(project: ProjectManifest) => void>();
    render(<AddAssetForm project="frostveil" onDone={onDone} onCancel={vi.fn()} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), '  Guardian statue ');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Kind' }), 'scene');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Column' }), 'refs');
    await userEvent.click(screen.getByRole('button', { name: 'Add asset' }));
    await vi.waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    const added = onDone.mock.calls[0]?.[0].assets?.find(a => a.name === 'Guardian statue');
    expect(added).toMatchObject({ kind: 'scene', stage: 'refs' });
  });

  it('cancels on Escape', async () => {
    const onCancel = vi.fn();
    render(<AddAssetForm project="frostveil" onDone={vi.fn()} onCancel={onCancel} />);
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), '{Escape}');
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
