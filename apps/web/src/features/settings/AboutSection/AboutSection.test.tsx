import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockUpdater } from '@/test/mock-ipc';
import { AboutSection } from '.';

afterEach(() => clearMocks());

describe('the About section', () => {
  it('shows the build, the machine and why updates are off', async () => {
    await openMockUpdater();
    render(<AboutSection active={false} />);
    const about = screen.getByRole('region', { name: 'About' });
    expect(await within(about).findByText('commit 4137061f2a, built 2026-10-27')).toBeVisible();
    expect(within(about).getByText('macOS 15.2')).toBeVisible();
    expect(within(about).getByText('aarch64')).toBeVisible();
    expect(within(about).getByText('2.11.5')).toBeVisible();
    expect(within(about).getByText('Not checked yet')).toBeVisible();
    expect(within(about).getByRole('status')).toHaveTextContent(
      'Updates arrive with the first public release'
    );
    expect(within(about).getByRole('button', { name: 'Check now' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('copies a version block with the checked Blender version', async () => {
    const user = userEvent.setup();
    await openMockUpdater();
    await useAppStore.getState().detectTools();
    render(<AboutSection active={false} />);
    await screen.findByText('commit 4137061f2a, built 2026-10-27');
    await user.click(screen.getByRole('button', { name: 'Copy version info' }));
    const text = await navigator.clipboard.readText();
    expect(text).toMatch(/^Maquette 0\.1\.0 \(commit 4137061f2a, built 2026-10-27\)\n/);
    expect(text).toContain('OS: macOS 15.2 (aarch64)');
    expect(text).toMatch(/Blender: \d/);
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();
  });

  it('opens the prefilled bug form, the docs and the source through the shell', async () => {
    const user = userEvent.setup();
    const handle = await openMockUpdater();
    const opened: string[] = [];
    mockIPC((cmd, args) => {
      if (cmd === 'app_open_link') opened.push(String((args as { url: string }).url));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    render(<AboutSection active={false} />);
    await screen.findByText('commit 4137061f2a, built 2026-10-27');
    await user.click(screen.getByRole('button', { name: 'Report a problem' }));
    await user.click(screen.getByRole('button', { name: 'Docs' }));
    await user.click(screen.getByRole('button', { name: 'Source' }));
    await user.click(screen.getByRole('button', { name: 'Release notes' }));
    await waitFor(() => expect(opened).toHaveLength(4));
    const report = new URL(opened[0]!);
    expect(report.pathname).toBe('/Shironex/maquette/issues/new');
    expect(report.searchParams.get('template')).toBe('bug_report.yml');
    expect(report.searchParams.get('version')).toBe('0.1.0 (4137061f2a)');
    expect(report.searchParams.get('os-version')).toBe('macOS 15.2 (aarch64)');
    expect(opened.slice(1)).toEqual([
      'https://shironex.github.io/maquette/',
      'https://github.com/Shironex/maquette',
      'https://github.com/Shironex/maquette/releases/tag/v0.1.0',
    ]);
  });

  it('says no usage data is sent', async () => {
    await openMockUpdater();
    render(<AboutSection active={false} />);
    expect(screen.getByText(/sends no usage data/)).toBeVisible();
  });
});
