import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import AgentPrompt from './AgentPrompt';

describe('AgentPrompt', () => {
  it('copies the prompt and says so', async () => {
    const user = userEvent.setup();
    const write = vi.spyOn(navigator.clipboard, 'writeText');
    render(
      <AgentPrompt title="Or let an agent start one" prompt="Make a canvas" hint="Over MCP" />
    );
    expect(screen.getByRole('region', { name: 'Or let an agent start one' })).toHaveTextContent(
      'Make a canvas'
    );
    await user.click(screen.getByRole('button', { name: 'Copy prompt' }));
    expect(write).toHaveBeenCalledWith('Make a canvas');
    expect(await screen.findByRole('status')).toHaveTextContent('Prompt copied');
  });

  it('says when the clipboard is not there', async () => {
    const user = userEvent.setup();
    vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValue(new Error('denied'));
    render(<AgentPrompt title="Ask an agent" prompt="Fill it" hint="Over MCP" />);
    await user.click(screen.getByRole('button', { name: 'Copy prompt' }));
    expect(await screen.findByRole('status')).toHaveTextContent('Could not copy');
  });
});
