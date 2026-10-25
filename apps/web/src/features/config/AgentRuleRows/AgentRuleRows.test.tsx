import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { defaultConfig } from '@/ipc/mock/fixtures';
import { AgentRuleRows } from '.';

describe('the agent rule rows', () => {
  it('flip one rule and keep the others', async () => {
    const config = defaultConfig();
    const onChange = vi.fn();
    render(<AgentRuleRows config={config} onChange={onChange} />);
    const before = config.agents.rules.neverCaptureScreen;
    await userEvent.click(screen.getByRole('switch', { name: 'Never capture the screen' }));
    expect(onChange).toHaveBeenCalledWith({
      ...config,
      agents: {
        ...config.agents,
        rules: { ...config.agents.rules, neverCaptureScreen: !before },
      },
    });
  });

  it('show the details only when roomy', () => {
    const detail = 'Agents run Blender in the background and never open its window';
    const { rerender } = render(<AgentRuleRows config={defaultConfig()} onChange={() => {}} />);
    expect(screen.getByText(detail)).toBeInTheDocument();
    rerender(<AgentRuleRows config={defaultConfig()} onChange={() => {}} density="compact" />);
    expect(screen.queryByText(detail)).toBeNull();
  });
});
