import { act, render, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePackStore } from '@/state/use-pack-store';
import { PackSheetHost } from '.';

vi.mock('@/features/library/PackSheet', () => new Promise(() => {}));

afterEach(() => {
  usePackStore.setState({ open: false });
  useAppStore.setState({ workspace: null });
});

describe('the pack sheet host while its code loads', () => {
  it('closes when another project opens before the code arrives', async () => {
    const project = (id: string) => ({ workspace: { project: { id } } }) as never;
    act(() => useAppStore.setState(project('one')));
    render(<PackSheetHost />);
    act(() => usePackStore.setState({ open: true }));
    act(() => useAppStore.setState(project('two')));
    await waitFor(() => expect(usePackStore.getState().open).toBe(false));
  });
});
