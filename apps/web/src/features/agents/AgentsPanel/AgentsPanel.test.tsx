import { render, screen, within } from '@testing-library/react';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { openAgentSession } from '../openAgentSession';
import { AgentsPanel } from '.';

afterEach(() => {
  clearMocks();
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
});
