import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { McpCall } from '@/ipc/client';
import { agentCalls } from '@/ipc/mock/agents';
import { ActivityRow } from '.';

const [first] = agentCalls(0);

function row(call: McpCall) {
  render(
    <ol>
      <ActivityRow call={call} />
    </ol>
  );
  return screen.getByRole('listitem');
}

describe('an activity row', () => {
  it('shows the detail for a call that went through', () => {
    const item = row({ ...first!, tool: 'run_pipeline', outcome: 'ok', detail: 'frostveil' });
    expect(item).toHaveTextContent('run_pipeline');
    expect(item).toHaveTextContent('frostveil');
    expect(item).toHaveTextContent(/ok$/);
  });

  it('shows the message for a refused call', () => {
    const item = row({
      ...first!,
      outcome: 'refused',
      detail: 'ignored',
      message: 'A request came without the right token.',
    });
    expect(item).toHaveTextContent('A request came without the right token.');
    expect(item).not.toHaveTextContent('ignored');
    expect(item).toHaveAttribute('title', 'A request came without the right token.');
  });
});
