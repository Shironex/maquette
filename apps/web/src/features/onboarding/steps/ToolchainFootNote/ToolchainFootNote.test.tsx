import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ToolchainFootNote } from '.';

afterEach(() => clearMocks());

describe('the toolchain foot note', () => {
  it('waits for the check, then counts the warnings', async () => {
    await openMockWorkspace('first-run');
    const { container } = render(<ToolchainFootNote />);
    expect(container).toBeEmptyDOMElement();
    await act(() => useAppStore.getState().detectTools());
    expect(screen.getByText('1 warning.')).toBeVisible();
    expect(screen.getByText(/Only Blender is required/)).toBeVisible();
  });

  it('says Blender is required when it is missing', async () => {
    await openMockWorkspace('no-blender');
    render(<ToolchainFootNote />);
    await act(() => useAppStore.getState().detectTools());
    expect(screen.getByText(/Blender is required. Install it/)).toBeVisible();
  });
});
