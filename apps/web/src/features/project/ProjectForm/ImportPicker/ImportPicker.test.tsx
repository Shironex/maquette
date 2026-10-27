import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ImportPicker } from '.';

afterEach(() => {
  cleanup();
  clearMocks();
});

describe('the import picker', () => {
  it('scans the folder and sums up what it found', async () => {
    await openMockWorkspace();
    render(<ImportPicker folder="~/scene" size="md" onChange={vi.fn()} onBrowse={vi.fn()} />);
    expect(
      await screen.findByText(
        '2 .blend files, latest scenes/veil_platform_v3.blend. 2 tools/ scripts.'
      )
    ).toBeVisible();
  });

  it('shows no note before a folder is picked', async () => {
    await openMockWorkspace();
    render(<ImportPicker folder="" size="md" onChange={vi.fn()} onBrowse={vi.fn()} />);
    expect(screen.getByLabelText('Folder to import')).toHaveValue('');
    expect(screen.queryByText(/\.blend files/)).toBeNull();
  });
});
