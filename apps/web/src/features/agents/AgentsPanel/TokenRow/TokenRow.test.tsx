import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { MOCK_TOKEN } from '@/ipc/mock/agents';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { openAgentSession } from '../../openAgentSession';
import { TokenRow } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
  useAppStore.setState({ notice: null });
});

describe('the token row', () => {
  it('masks the token until shown and copies it', async () => {
    const user = userEvent.setup();
    await openAgentSession('active');
    render(<TokenRow />);
    expect(screen.queryByText(MOCK_TOKEN)).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Show the token' }));
    expect(screen.getByText(MOCK_TOKEN)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Copy the token' }));
    expect(await navigator.clipboard.readText()).toBe(MOCK_TOKEN);
    expect(useAppStore.getState().notice?.title).toBe('Token copied');
  });

  it('asks the server for a new token', async () => {
    const calls = await openAgentSession('active');
    render(<TokenRow />);
    await userEvent.click(screen.getByRole('button', { name: 'New token' }));
    expect(calls).toContain('mcp_renew_token');
  });

  it('renders nothing before the server has made one', async () => {
    await openAgentSession(null);
    const { container } = render(<TokenRow />);
    expect(container).toBeEmptyDOMElement();
  });
});
