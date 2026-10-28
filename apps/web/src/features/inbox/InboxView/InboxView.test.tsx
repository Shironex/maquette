import { act, render, screen, within } from '@testing-library/react';
import { Component } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerMockFile } from '@/ipc/files';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import InboxView from './InboxView';

/** The first file the preview opens fails, as a download still in flight does. */
let failures = 1;
vi.mock('@/features/viewer/ModelPreview', () => ({
  default: class Preview extends Component<{ onError: (message: string) => void }> {
    componentDidMount() {
      if (failures <= 0) return;
      failures -= 1;
      this.props.onError('The file ends early.');
    }
    render() {
      return null;
    }
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

  it('renders only the rows in view of a long inbox', async () => {
    await openMockWorkspace();
    await useInboxStore.getState().load();
    const snapshot = useInboxStore.getState().snapshot!;
    const one = snapshot.items[0]!;
    const items = Array.from({ length: 200 }, (_, i) => ({ ...one, id: `copy-${i}` }));
    useInboxStore.setState({ snapshot: { ...snapshot, items } });
    render(<InboxView />);
    const list = screen.getByRole('region', { name: 'Models in the inbox' });
    expect(within(list).getByText('200 models waiting')).toBeVisible();
    expect(within(list).getAllByRole('listitem').length).toBeLessThan(40);
  });
});
