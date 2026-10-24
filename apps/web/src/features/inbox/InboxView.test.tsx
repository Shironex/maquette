import { act, render, screen } from '@testing-library/react';
import { useEffect } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import { useInboxStore } from '@/state/inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { InboxView } from './InboxView';

/** The first file the preview opens fails, as a download still in flight does. */
let failures = 1;
vi.mock('@/features/viewer/ModelPreview', () => ({
  default: function Preview({ onError }: { onError: (message: string) => void }) {
    useEffect(() => {
      if (failures > 0) {
        failures -= 1;
        onError('The file ends early.');
      }
    }, [onError]);
    return null;
  },
}));

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
});

describe('the inbox preview', () => {
  it('starts over when the file under the same name changes', async () => {
    await openMockWorkspace();
    await useInboxStore.getState().load();
    const item = useInboxStore.getState().snapshot!.items.find(i => i.state === 'ready')!;
    registerMockFile(item.file, '/mock/model.glb');
    useInboxStore.setState({ selected: item.id });
    render(<InboxView />);
    expect(await screen.findByText('The file ends early.')).toBeInTheDocument();

    // The watcher lists the finished download: same name, more bytes.
    act(() => {
      const snapshot = useInboxStore.getState().snapshot!;
      useInboxStore.setState({
        snapshot: {
          ...snapshot,
          items: snapshot.items.map(i =>
            i.id === item.id
              ? { ...i, bytes: (i.bytes ?? 0) + 1024, modifiedMs: (i.modifiedMs ?? 0) + 5000 }
              : i
          ),
        },
      });
    });
    expect(screen.queryByText('The file ends early.')).toBeNull();
    expect(screen.queryByText('No preview for this file')).toBeNull();
  });
});
