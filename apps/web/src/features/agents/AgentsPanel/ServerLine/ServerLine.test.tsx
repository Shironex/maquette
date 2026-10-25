import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ServerLine } from '.';

describe('the server line', () => {
  it('says the server is starting once enabled with no status yet', () => {
    render(<ServerLine status={null} enabled />);
    expect(screen.getByRole('status')).toHaveTextContent('Starting the server');
  });

  it('says how to turn it on when off', () => {
    render(<ServerLine status={null} enabled={false} />);
    expect(screen.getByRole('status')).toHaveTextContent(
      'Off. Turn it on to let Claude Code and Codex use Maquette over MCP.'
    );
  });
});
