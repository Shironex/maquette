import { mcpSnippet, type McpAgent } from '@/state/settings-form';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { copyText } from '../copyText';

/**
 * The server's bearer token (until it exists the copy buttons wait) and the
 * copy action that puts an agent's config, token included, on the clipboard.
 */
export function useMcpCopy(port: number) {
  const notify = useAppStore(s => s.notify);
  const token = useAgentsStore(s => s.status?.token ?? null);
  const copy = async (agent: McpAgent) => {
    if (!token) return;
    const ok = await copyText(mcpSnippet(agent, port, token));
    const name = agent === 'claude' ? 'Claude Code' : 'Codex';
    notify(
      ok
        ? {
            status: 'done',
            title: `${name} config copied`,
            detail:
              agent === 'claude'
                ? 'Run it in a terminal. It holds the token, so keep it private.'
                : 'Paste it into ~/.codex/config.toml. It holds the token, so keep it private.',
          }
        : {
            status: 'failed',
            title: 'Could not copy',
            detail: 'The clipboard is not available here.',
          }
    );
  };
  return { token, copy };
}
