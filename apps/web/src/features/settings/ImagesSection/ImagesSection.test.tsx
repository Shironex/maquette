import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { WorkspaceConfig } from '@/ipc/client';
import { createBackend } from '@/ipc/mock/backend';
import { useImageApiStore } from '@/state/use-image-api-store';
import { clearMocks } from '@/test/mock-ipc';
import { ImagesSection } from '.';

const store = useImageApiStore.getState();

afterEach(async () => {
  // Unmount first and let the listener unregister through the mock.
  cleanup();
  await act(async () => undefined);
  clearMocks();
  useImageApiStore.setState(store, true);
});

async function open(images: string | null) {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, images });
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
    shouldMockEvents: true,
  });
  const boot = (await handle('app_bootstrap')) as { workspace: { config: WorkspaceConfig } };
  return boot.workspace.config;
}

describe('the image generation settings', () => {
  it('start on Codex for every kind, with no key', async () => {
    render(<ImagesSection active config={await open(null)} onChange={vi.fn()} />);
    expect(screen.getByRole('region', { name: /Image generation/ })).toBeInTheDocument();
    expect(
      screen.getAllByRole('combobox', { name: /Concepts|Object refs|Other views|Clean background/ })
    ).toHaveLength(4);
    expect(await screen.findByLabelText('OpenAI API key')).toBeInTheDocument();
  });

  it('shows the saved key, the month and switches a kind to OpenAI Images', async () => {
    const onChange = vi.fn();
    render(<ImagesSection active config={await open('on')} onChange={onChange} />);
    expect(await screen.findByText(/^•{8}k7Qm$/)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('$3.42')).toBeInTheDocument());
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Other views' }),
      'openai-images'
    );
    const next = onChange.mock.calls.at(-1)?.[0] as WorkspaceConfig;
    expect(next.lab.images?.backends).toMatchObject({
      concepts: 'openai-images',
      otherViews: 'openai-images',
    });
  });
});
