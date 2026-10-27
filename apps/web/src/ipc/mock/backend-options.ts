import type {
  Arrival,
  ConceptEvent,
  ErrorPayload,
  McpStatus,
  PipelineEvent,
  ProjectManifest,
  ProviderEvent,
  ServerStatus,
} from '../client';
import { HOME } from './fixtures';

export type Args = Record<string, unknown>;

/**
 * What shapes the mock backend, read from the URL by `install.ts`.
 *
 * Scenarios (`?scenario=`):
 * - `first-run` (default): no workspace yet, toktx missing.
 * - `configured`: a workspace with Moon Island is open.
 * - `no-blender`: like first-run, but Blender is missing.
 * - `running`: configured, with run 25 live at Export GLB.
 * - `idle`: configured, the last run passed.
 *
 * Configured workspaces hold Moon Island, Rooftop cinema (a Room) and
 * Lantern keeper (a Character showcase); `?projects=many` adds five more so
 * the project switcher shows its search.
 *
 * `?pace=` sets the milliseconds between streamed log lines (default 260).
 * `?lab=running` holds two concept images of the porch direction mid job.
 * `?agents=active` turns the MCP server on, with an agent's activity, a
 * run waiting at Ship for approval and a send to Tripo waiting the same way;
 * `?agents=batch` makes that send a batch of three under one approval.
 * `?handoff=batch` adds the landing workbench concept, six object refs made
 * from it and a By hand batch of them in mixed states.
 * `?inbox=redo` links the large sakura and the character in the inbox to
 * the sends they came from (Meshy with a texture prompt, rigged Tripo) and
 * saves a Meshy key too, for Redo. The credits ledger holds the live sends
 * and a month of earlier ones. `?arrival=named|unmatched` drops a download
 * for the Downloads watcher (see `create-downloads-handlers.ts`).
 * `?built=demo` adds two built assets to Moon Island (see
 * `create-built-handlers.ts`), `?motion=` models that move (see
 * `create-motion-handlers.ts`), `?page=` holds the Page view's dev server
 * (see `create-page-handlers.ts`), `?timeline=` the Timeline view's tracks
 * (see `create-timeline-handlers.ts`), `?paths=` the Paths editor's paths
 * (see `create-paths-handlers.ts`), `?game=` a game project (see
 * `create-game-handlers.ts`), `?library=demo` the library's entries (see
 * `create-library-handlers.ts`), `?inspect=many` a long compare menu (see
 * `many-assets-fixtures.ts`). Configured scenarios also open with
 * Send to 3D jobs (see `providers.ts`).
 */
export interface BackendOptions {
  scenario: string;
  /** Milliseconds a toolchain check takes, so the checking state is visible. */
  probeDelay: number;
  /** Milliseconds between streamed log lines. */
  pace?: number;
  /** Delivers runner, inbox and concept events to the UI. */
  emit?: (
    event:
      | { name: 'pipeline'; payload: PipelineEvent }
      | { name: 'inbox' }
      | { name: 'concept'; payload: ConceptEvent }
      | { name: 'mcp'; payload: McpStatus }
      | { name: 'provider'; payload: ProviderEvent }
      | { name: 'downloads'; payload: Arrival }
      | { name: 'built'; project: string; manifest?: ProjectManifest }
      | { name: 'page'; payload: ServerStatus }
  ) => void;
  /** `active`: the MCP server is on, mid agent session. */
  agents?: string | null;
  /** `running` holds two concept jobs mid way, for screenshots. */
  lab?: string | null;
  /** Milliseconds one fake concept image takes (default 4000). */
  conceptMs?: number;
  /** `many`: eight projects instead of three. */
  projects?: string | null;
  /** `batch`: the workbench objects and their By hand batch. */
  handoff?: string | null;
  /** `redo`: two inbox models linked to their sends, and both keys saved. */
  inbox?: string | null;
  /** `named` or `unmatched`: a download for the Downloads watcher. */
  arrival?: string | null;
  /** `demo`: two built assets on Moon Island, one current and one stale. */
  built?: string | null;
  /** `demo`, `rest`, `over` or `still`: models that move (see create-motion-handlers.ts). */
  motion?: string | null;
  /** `starting`, `failed` or `no-site` holds the Page view's dev server there. */
  page?: string | null;
  /** `demo` or `empty`: the Timeline view's tracks (see create-timeline-handlers.ts). */
  timeline?: string | null;
  /** `demo` or `empty`: the Paths editor's paths (see create-paths-handlers.ts). */
  paths?: string | null;
  /** `crypt`, `import` or `maze`: a game project is open (see create-game-handlers.ts). */
  game?: string | null;
  /** `demo`: library entries, two of them with an update for a project (see library-fixtures.ts). */
  library?: string | null;
  /** `many`: Moon Island gets 49 more versioned assets, so the Inspector's compare menu is long. */
  inspect?: string | null;
  /** Serves files the backend "writes", since mock mode has no disk. */
  files?: {
    /** Serves whatever `from` serves at `path` too. */
    alias: (path: string, from: string) => void;
    /** Serves `bytes` (a PNG) at `path`. */
    store: (path: string, bytes: Uint8Array) => void;
    /** Tiles the images served at `sources` into one PNG served at `path`. */
    sheet?: (path: string, sources: string[]) => Promise<void>;
  };
}

export function fail(code: string, message: string): never {
  const payload: ErrorPayload = { code, message };
  throw payload;
}

export const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

export function expand(path: string): string {
  return path.replace(/^~(?=$|\/)/, HOME);
}

export function display(path: string): string {
  return path.startsWith(HOME) ? `~${path.slice(HOME.length)}` : path;
}

export function slugify(name: string): string {
  return (
    name
      .toLowerCase()
      .normalize('NFKD')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'project'
  );
}
