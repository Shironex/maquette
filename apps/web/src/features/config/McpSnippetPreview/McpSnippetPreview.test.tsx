import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { McpSnippetPreview } from '.';

describe('the MCP snippet preview', () => {
  it('shows the command for the port with the token left out', () => {
    const { container } = render(<McpSnippetPreview port={4123} />);
    const text = container.querySelector('pre')?.textContent;
    expect(text).toContain('claude mcp add --transport http maquette http://127.0.0.1:4123/mcp');
    expect(text).toContain('Bearer <token from Settings > Agents>"');
  });
});
