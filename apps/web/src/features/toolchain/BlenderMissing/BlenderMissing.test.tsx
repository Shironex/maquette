import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands, type ToolReport } from '@/ipc/client';
import { blenderInstallCommand, BLENDER_DOWNLOAD } from '@/state/toolchain';
import { useAppStore } from '@/state/use-app-store';
import { BlenderMissing } from '.';

const initial = useAppStore.getState();

afterEach(() => {
  vi.restoreAllMocks();
  useAppStore.setState(initial, true);
});

function blender(status: ToolReport['status']): ToolReport {
  return {
    id: 'blender',
    label: 'Blender',
    required: true,
    status,
    source: 'path',
    path: null,
    version: null,
    detail: null,
    hint: null,
  } as ToolReport;
}

function withBlender(status: ToolReport['status']) {
  const detectTools = vi.fn(async () => undefined);
  useAppStore.setState({
    detectTools,
    tools: { ...initial.tools, reports: [blender(status)], checking: false },
  });
  return detectTools;
}

describe('the Blender screen', () => {
  it('stays away while Blender works', () => {
    withBlender('found');
    render(<BlenderMissing />);
    expect(screen.queryByRole('region')).toBeNull();
  });

  it('opens blender.org', async () => {
    withBlender('missing');
    const open = vi.spyOn(commands, 'appOpenLink').mockResolvedValue(null);
    render(<BlenderMissing />);
    await userEvent.click(screen.getByRole('button', { name: 'Get Blender' }));
    expect(open).toHaveBeenCalledWith(BLENDER_DOWNLOAD);
  });

  it('looks again when the window comes back, only while it is missing', async () => {
    const detectTools = withBlender('missing');
    const { unmount } = render(<BlenderMissing />);
    act(() => void window.dispatchEvent(new Event('focus')));
    await waitFor(() => expect(detectTools).toHaveBeenCalledOnce());
    unmount();
    const found = withBlender('found');
    render(<BlenderMissing />);
    act(() => void window.dispatchEvent(new Event('focus')));
    expect(found).not.toHaveBeenCalled();
  });

  it('says a broken copy did not run', () => {
    withBlender('broken');
    render(<BlenderMissing />);
    expect(screen.getByRole('region', { name: 'Blender did not run' })).toBeVisible();
  });

  it('says a saved path is gone without asking for an install, when another copy works', () => {
    withBlender('found');
    const { tools } = useAppStore.getState();
    useAppStore.setState({
      tools: { ...tools, reports: [{ ...blender('found'), warning: 'stale' }] },
    });
    render(<BlenderMissing />);
    const screenRegion = screen.getByRole('region', { name: 'Your saved Blender path is gone' });
    expect(screenRegion).toBeVisible();
    expect(screenRegion).not.toHaveTextContent(/did not run|install|4\.2/i);
    expect(screen.queryByRole('button', { name: 'Get Blender' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Copy the install command' })).toBeNull();
  });

  it('names a command for the computers with one', () => {
    expect(blenderInstallCommand('mac')).toBe('brew install --cask blender');
    expect(blenderInstallCommand('windows')).toBe('winget install BlenderFoundation.Blender');
    expect(blenderInstallCommand('linux')).toBeNull();
  });
});
