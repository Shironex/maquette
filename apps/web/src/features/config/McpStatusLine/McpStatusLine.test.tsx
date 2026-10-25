import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { McpStatusLine } from '.';

describe('the MCP status line', () => {
  it('names the port when the server is on and says how to turn it on when off', () => {
    const config = defaultConfig();
    const on = { ...config, agents: { ...config.agents, mcpEnabled: true, mcpPort: 4555 } };
    const { rerender } = render(<McpStatusLine config={on} />);
    expect(
      screen.getByText(/^Starts on 127\.0\.0\.1:4555 once the workspace exists/)
    ).toBeInTheDocument();
    const off = { ...config, agents: { ...config.agents, mcpEnabled: false } };
    rerender(<McpStatusLine config={off} />);
    expect(
      screen.getByText('Off. You can turn it on later in Settings > Agents.')
    ).toBeInTheDocument();
  });
});
