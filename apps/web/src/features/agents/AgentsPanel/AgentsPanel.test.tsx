import { render, screen, within } from '@testing-library/react';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { forgetToolGroups } from '@/features/config/ToolGroupRows/ToolGroupRows.hooks';
import { useAppStore } from '@/state/use-app-store';
import { openAgentSession } from '../openAgentSession';
import { AgentsPanel } from '.';

afterEach(() => {
  clearMocks();
  forgetToolGroups();
  useAgentsStore.setState({ status: null, deciding: null });
});

function renderPanel() {
  const config = useAppStore.getState().workspace!.config;
  return render(<AgentsPanel config={config} onChange={() => {}} errors={{}} />);
}

describe('Settings > Agents', () => {
  it('lists the server, token, waiting approvals and activity mid session', async () => {
    await openAgentSession('batch');
    renderPanel();
    expect(screen.getByText(/^Listening on /)).toBeInTheDocument();
    expect(screen.getByText('Bearer token')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Waiting for you' })).toBeInTheDocument();
    expect(screen.getAllByRole('alertdialog')).toHaveLength(2);
    const activity = screen.getByRole('list', { name: 'MCP activity' });
    expect(within(activity).getAllByRole('listitem').length).toBeGreaterThan(0);
  });

  it('shows no token, approvals or calls while the server is off', async () => {
    await openAgentSession(null);
    renderPanel();
    expect(
      screen.getByText('Off. Turn it on to let Claude Code and Codex use Maquette over MCP.')
    ).toBeInTheDocument();
    expect(screen.queryByText('Bearer token')).toBeNull();
    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(
      screen.getByText('No calls yet. Each tool call an agent makes shows up here.')
    ).toBeInTheDocument();
  });

  it('lists the tool groups from the server, each with a switch', async () => {
    const calls = await openAgentSession('active');
    renderPanel();
    expect(await screen.findByRole('switch', { name: 'Avatar and rig tools' })).toBeChecked();
    expect(screen.getByRole('switch', { name: 'Start tools' })).toBeDisabled();
    expect(screen.getByRole('heading', { name: 'Tool groups' })).toBeInTheDocument();
    expect(calls).toContain('mcp_tool_groups');
  });
});
