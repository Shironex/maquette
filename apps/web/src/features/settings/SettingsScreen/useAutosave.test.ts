import { act, renderHook, waitFor } from '@testing-library/react';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { patchSection } from '@/state/settings-form';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { useAutosave } from './useAutosave';

afterEach(() => clearMocks());

describe('the settings autosave', () => {
  it('still saves a change made right before Settings closes', async () => {
    await openMockWorkspace();
    const { result, unmount } = renderHook(() => useAutosave());
    const before = useAppStore.getState().workspace!.config;
    const next = patchSection(before, 'budgets', { sceneMb: (before.budgets.sceneMb ?? 3) + 1 });
    act(() => result.current.setDraft(next));
    expect(result.current.status).toBe('pending');
    // Board is clicked before the save timer fires.
    unmount();
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.config.budgets.sceneMb).toBe(next.budgets.sceneMb)
    );
  });

  it('does not save an invalid draft when Settings closes', async () => {
    await openMockWorkspace();
    const { result, unmount } = renderHook(() => useAutosave());
    const before = useAppStore.getState().workspace!.config;
    act(() => result.current.setDraft(patchSection(before, 'budgets', { sceneMb: -1 })));
    unmount();
    await new Promise(r => setTimeout(r, 600));
    expect(useAppStore.getState().workspace?.config.budgets.sceneMb).toBe(before.budgets.sceneMb);
  });

  it('does not write the config when only the open project changed', async () => {
    const handle = await openMockWorkspace();
    const calls: string[] = [];
    mockIPC((cmd, args) => {
      calls.push(cmd);
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    const { result } = renderHook(() => useAutosave());
    await act(() => useProjectsStore.getState().switchTo('rooftop-cinema'));
    expect(result.current.status).toBe('saved');
    await new Promise(r => setTimeout(r, 600));
    expect(calls).not.toContain('workspace_save_config');
  });
});
