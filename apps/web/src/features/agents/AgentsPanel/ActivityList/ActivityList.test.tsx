import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAgentsStore } from '@/state/use-agents-store';
import { openAgentSession } from '../../openAgentSession';
import { ActivityList } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
});

describe('the activity list', () => {
  it('clears the calls through the server', async () => {
    const calls = await openAgentSession('active');
    const { rerender } = render(<ActivityList status={useAgentsStore.getState().status} />);
    expect(screen.getByRole('list', { name: 'MCP activity' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Clear' }));
    expect(calls).toContain('mcp_clear_activity');
    await waitFor(() => expect(useAgentsStore.getState().status?.calls).toEqual([]));
    rerender(<ActivityList status={useAgentsStore.getState().status} />);
    expect(
      screen.getByText('No calls yet. Each tool call an agent makes shows up here.')
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Clear' })).toBeNull();
  });
});
