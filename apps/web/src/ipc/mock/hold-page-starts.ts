/**
 * Mock mode's page preview gate (`?agents=preview`): while a card waits, a
 * start from the app leaves the server idle and Preview on site refuses, as
 * the real gate does, and a yes starts the server.
 */
import { fail } from './backend-options';

type Args = Record<string, unknown>;
type Handlers = Record<string, (args: Args) => unknown>;

export function holdPageStarts(
  page: Handlers,
  agents: { handlers: Handlers; previewWaits: (project: string) => boolean }
): Handlers {
  const waits = (args: Args) => agents.previewWaits(String(args.project));
  return {
    ...page,
    page_server_start: args =>
      waits(args) ? page.page_get?.(args) : page.page_server_start?.(args),
    site_preview: args => {
      if (waits(args)) {
        fail(
          'invalid',
          'An agent set this site repo, so its dev command waits for your approval on the card. Approve it to start the server, then use Preview on site again.'
        );
      }
      return page.site_preview?.(args);
    },
    mcp_decide: args => {
      // The one card mock mode raises is on its open project.
      const project = { project: 'frostveil' };
      const held = waits(project);
      const answer = agents.handlers.mcp_decide?.(args);
      if (held && !waits(project) && args.approve) page.page_server_start?.(project);
      return answer;
    },
  };
}
