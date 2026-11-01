import { cleanup, render, renderHook, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ToolReport } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { ToolList, useToolHeadline } from '.';

const initial = useAppStore.getState();

afterEach(() => {
  cleanup();
  useAppStore.setState(initial, true);
});

function report(id: ToolReport['id'], over: Partial<ToolReport> = {}): ToolReport {
  return {
    id,
    label: id,
    required: id === 'blender',
    status: 'found',
    source: 'path',
    path: null,
    version: '1.0',
    detail: null,
    hint: null,
    warning: null,
    ...over,
  };
}

const tools = (over: Partial<ReturnType<typeof useAppStore.getState>['tools']>) =>
  useAppStore.setState({
    tools: { reports: [], checking: false, busyTool: null, checkedAt: null, ...over },
  });

describe('the tool list', () => {
  it('shows every tool as checking before the first report', () => {
    tools({ checking: true });
    render(<ToolList density="roomy" />);
    expect(screen.getByText('Blender')).toBeVisible();
    expect(screen.getByText('KTX-Software toktx')).toBeVisible();
    expect(screen.getAllByText('checking...')).toHaveLength(6);
    expect(screen.getByRole('button', { name: 'Locate Blender' })).toBeDisabled();
  });

  it('shows the version and locates a tool through the store', async () => {
    const locateTool = vi.fn(async () => {});
    useAppStore.setState({ locateTool });
    tools({
      reports: [
        report('blender', {
          version: '4.2',
          path: '/Applications/Blender.app/Contents/MacOS/Blender',
        }),
        report('ffmpeg', {
          status: 'missing',
          version: null,
          detail: 'not found',
          hint: 'brew install ffmpeg',
        }),
      ],
    });
    render(<ToolList density="roomy" />);
    expect(screen.getByText('4.2 at /Applications/Blender.app')).toBeVisible();
    expect(screen.getByText('brew install ffmpeg')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Locate ffmpeg' }));
    expect(locateTool).toHaveBeenCalledWith('ffmpeg');
  });

  it('uses the short names and drops the hint when compact', () => {
    tools({ reports: [report('toktx', { hint: 'install it' })] });
    render(<ToolList density="compact" />);
    expect(screen.queryByText('KTX-Software toktx')).toBeNull();
    expect(screen.queryByText('install it')).toBeNull();
  });
});

describe('the tool headline', () => {
  it('counts the found tools', () => {
    tools({ reports: [report('blender'), report('ffmpeg', { status: 'missing' })] });
    const { result } = renderHook(() => useToolHeadline());
    expect(result.current.headline).toBe('1 of 2 found');
    expect(result.current.checked).toBe('');
  });

  it('says it is checking before any report', () => {
    tools({ checking: true });
    const { result } = renderHook(() => useToolHeadline());
    expect(result.current.headline).toBe('Checking your tools');
    expect(result.current.checked).toBe('Checking...');
  });
});
