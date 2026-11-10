import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { WorkspaceConfig } from '@/ipc/client';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { MOCK_TOOL_GROUPS } from '@/ipc/mock/mock-tool-groups';
import { ToolGroupRows } from '.';
import { forgetToolGroups, offeredCount, withGroup } from './ToolGroupRows.hooks';

function withOff(ids: string[]): WorkspaceConfig {
  const config = defaultConfig();
  return {
    ...config,
    agents: { ...config.agents, rules: { ...config.agents.rules, toolGroupsOff: ids } },
  };
}

beforeEach(() => {
  forgetToolGroups();
  mockIPC(cmd => (cmd === 'mcp_tool_groups' ? MOCK_TOOL_GROUPS : null));
});

afterEach(() => clearMocks());

describe('the tool group rows', () => {
  it('list every group with its count, all on by default', async () => {
    render(<ToolGroupRows config={defaultConfig()} onChange={() => {}} />);
    const avatar = await screen.findByRole('switch', { name: 'Avatar and rig tools' });
    expect(avatar).toBeChecked();
    expect(screen.getAllByRole('switch')).toHaveLength(MOCK_TOOL_GROUPS.length);
    expect(screen.getByText('156 of 156 tools offered')).toBeInTheDocument();
    expect(screen.getByText('VRM avatars and the rig doctor')).toBeInTheDocument();
    expect(screen.getByText(/keeps its list until it reconnects/)).toBeInTheDocument();
  });

  it('turn a group off and on, keeping the others', async () => {
    const onChange = vi.fn();
    const config = withOff(['game']);
    render(<ToolGroupRows config={config} onChange={onChange} />);
    const avatar = await screen.findByRole('switch', { name: 'Avatar and rig tools' });
    await userEvent.click(avatar);
    expect(onChange).toHaveBeenLastCalledWith(withGroup(config, 'avatar-rig', false));
    expect(onChange.mock.lastCall?.[0].agents.rules.toolGroupsOff).toEqual(['game', 'avatar-rig']);
    expect(screen.getByRole('switch', { name: 'Game projects tools' })).not.toBeChecked();
    expect(screen.getByText('151 of 156 tools offered')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Game projects tools' }));
    expect(onChange.mock.lastCall?.[0].agents.rules.toolGroupsOff).toEqual([]);
  });

  it('never turn the start and project groups off', async () => {
    const onChange = vi.fn();
    // A hand edited file may name them; they stay on all the same.
    render(<ToolGroupRows config={withOff(['start', 'project'])} onChange={onChange} />);
    for (const name of ['Start tools', 'Projects tools']) {
      const core = await screen.findByRole('switch', { name });
      expect(core).toBeChecked();
      expect(core).toBeDisabled();
      await userEvent.click(core);
    }
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getAllByText(/always on/)).toHaveLength(2);
  });

  it('say so when the groups cannot be read', async () => {
    mockIPC(() => Promise.reject(new Error('The server is gone.')));
    render(<ToolGroupRows config={defaultConfig()} onChange={() => {}} />);
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/gone/));
    expect(screen.queryByRole('switch')).toBeNull();
  });

  it('count the offered tools from the groups that are on', () => {
    expect(offeredCount(MOCK_TOOL_GROUPS, ['avatar-rig', 'start'])).toEqual({
      offered: 149,
      total: 156,
    });
  });
});
