import { describe, expect, it } from 'vitest';
import { agentName } from './agent-name';

describe('agent names as people read them', () => {
  it('reads known MCP clients as their product', () => {
    expect(agentName('claude-code')).toBe('Claude Code');
    expect(agentName('claude-ai')).toBe('Claude');
    expect(agentName('codex-mcp-client')).toBe('Codex');
    expect(agentName('cursor-vscode')).toBe('Cursor');
    expect(agentName('Visual Studio Code')).toBe('VS Code');
  });

  it("reads an SDK's default name as an agent, other raw ids beside it", () => {
    expect(agentName('rmcp')).toBe('An agent');
    expect(agentName('mcp')).toBe('An agent');
    expect(agentName('mqtest')).toBe('An agent (mqtest)');
    expect(agentName('')).toBe('An agent');
    expect(agentName(undefined)).toBe('An agent');
  });

  it('keeps a name that already reads as one', () => {
    expect(agentName('Motion agent')).toBe('Motion agent');
    expect(agentName('Frostveil Helper')).toBe('Frostveil Helper');
  });
});
