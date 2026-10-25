import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { ToolchainStep } from '.';

afterEach(() => clearMocks());

describe('the toolchain step', () => {
  it('lists the detected tools', async () => {
    await openMockWorkspace('first-run');
    render(<ToolchainStep />);
    expect(screen.getByRole('region', { name: 'Toolchain' })).toBeVisible();
    await act(() => useAppStore.getState().detectTools());
    expect(screen.getByText('KTX-Software toktx')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Re-check all' })).toBeVisible();
  });
});
