import { mcpUrl } from '@/state/settings-form';

export interface McpSnippetPreviewProps {
  port: number;
}

/** The Claude Code command, with the token left out, as a preview. */
export default function McpSnippetPreview({ port }: McpSnippetPreviewProps) {
  return (
    <pre className="m-0 overflow-x-auto rounded-[8px] border border-line bg-app px-3.5 py-3 font-mono text-[12px] leading-[19px] whitespace-pre text-fg-dim">
      <span className="text-clay-text">claude mcp add</span>
      {` --transport http maquette ${mcpUrl(port)} \\\n  --header "Authorization: Bearer `}
      <span className="text-fg-mute">{'<token from Settings > Agents>'}</span>
      {'"'}
    </pre>
  );
}
