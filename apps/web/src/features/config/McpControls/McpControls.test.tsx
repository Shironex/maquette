import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { McpControls } from '.';

afterEach(() => {
  clearMocks();
  useAgentsStore.setState({ status: null });
  useAppStore.setState({ notice: null });
});

async function withToken(token: string | null) {
  await openMockWorkspace();
  const status = await commands.mcpStatus();
  useAgentsStore.setState({ status: { ...status, token } });
}

describe('the MCP controls', () => {
  it('copy the Claude Code command with the token', async () => {
    const user = userEvent.setup();
    await withToken('tok-123');
    const config = defaultConfig();
    render(<McpControls config={config} onChange={() => {}} />);
    await user.click(screen.getByRole('button', { name: 'Copy config for Claude Code' }));
    const copied = await navigator.clipboard.readText();
    expect(copied).toContain(`http://127.0.0.1:${config.agents.mcpPort}/mcp`);
    expect(copied).toContain('Bearer tok-123');
    expect(useAppStore.getState().notice?.title).toBe('Claude Code config copied');
  });

  it('wait for the token before copying', async () => {
    await withToken(null);
    render(<McpControls config={defaultConfig()} onChange={() => {}} density="compact" />);
    expect(screen.getByRole('button', { name: 'Codex config' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('hide the copy buttons and show the port error', async () => {
    await withToken(null);
    render(
      <McpControls
        config={defaultConfig()}
        onChange={() => {}}
        errors={{ mcpPort: 'Use a port from 1024 to 65535' }}
        showCopy={false}
      />
    );
    expect(screen.queryByRole('button', { name: /config/ })).toBeNull();
    expect(screen.getByText('Use a port from 1024 to 65535')).toBeInTheDocument();
  });
});
