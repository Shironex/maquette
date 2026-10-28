import type {
  Arrival,
  ConceptEvent,
  ErrorPayload,
  McpStatus,
  PipelineEvent,
  ProjectManifest,
  ProviderEvent,
  ServerStatus,
  UpdaterEvent,
} from '../client';
import { HOME } from './fixtures';

export type Args = Record<string, unknown>;

/**
 * What shapes the mock backend, read from the URL by `install.ts`.
 *
 * Scenarios (`?scenario=`):
 * - `first-run` (default): no workspace yet, toktx missing.
 * - `configured`: a workspace with Frostveil is open.
 * - `no-blender`: like first-run, but Blender is missing.
 * - `running`: configured, with run 25 live at Export GLB.
 * - `idle`: configured, the last run passed.
 *
 * Configured workspaces hold Frostveil, Frost cathedral (a Room) and
 * Elsheria showcase (a Character showcase); `?projects=many` adds five more so
 * the project switcher shows its search.
 *
 * `?pace=` sets the milliseconds between streamed log lines (default 260).
 * `?lab=running` holds two concept images of the arena direction mid job;
 * `?lab=limited` adds a rate limited job and `?lab=usage` three waiting for
 * the Codex usage limit. `?lab=refs` tags the guardian statues' refs for
 * the refs sheet and the ref warnings (see `create-refs-handlers.ts`).
 * `?agents=active` turns the MCP server on, with an agent's activity, a
 * run waiting at Ship for approval and a send to Tripo waiting the same way;
 * `?agents=batch` makes that send a batch of three under one approval.
 * `?handoff=batch` adds the veil gate props concept, six object refs made
 * from it and a By hand batch of them in mixed states.
 * `?style=board` gives Frostveil a style board (see `style-board.ts`).
 * `?inbox=redo` links the veil gate and Elsheria in the inbox to
 * the sends they came from (Meshy with a texture prompt, rigged Tripo) and
 * saves a Meshy key too, for Redo. The credits ledger holds the live sends
 * and a month of earlier ones. `?arrival=named|unmatched|look` drops downloads
 * for the Downloads watcher (see `create-downloads-handlers.ts`).
 * `?built=demo` adds two built assets to Frostveil (see
 * `create-built-handlers.ts`), `?motion=` models that move (see
 * `create-motion-handlers.ts`), `?parts=demo` props with parts (see
 * `create-parts-handlers.ts`), `?page=` holds the Page view's dev server
 * (see `create-page-handlers.ts`), `?timeline=` the Timeline view's tracks
 * (see `create-timeline-handlers.ts`), `?paths=` the Paths editor's paths
 * (see `create-paths-handlers.ts`), `?game=` a game project (see
 * `create-game-handlers.ts`), `?library=demo` the library's entries (see
 * `create-library-handlers.ts`), `?inspect=many` a long compare menu (see
 * `many-assets-fixtures.ts`), `?ship=hand-built` the website Ship settings
 * (see `create-ship-handlers.ts`), `?updater=` the app updater turned on (see
 * `create-app-handlers.ts`), `?pipeline=many` a runner with 24 models (see
 * `pipeline-many.ts`). Configured scenarios also open with
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
      | { name: 'updater'; payload: UpdaterEvent }
  ) => void;
  /** `active`: the MCP server is on, mid agent session. */
  agents?: string | null;
  /** `running` holds two concept jobs mid way, for screenshots. */
  lab?: string | null;
  /** Milliseconds one fake concept image takes (default 4000). */
  conceptMs?: number;
  /** `many`: eight projects instead of three. */
  projects?: string | null;
  /** `batch`: the veil gate objects and their By hand batch. */
  handoff?: string | null;
  /** `board`: Frostveil has a style board, and its directions say which carried it. */
  style?: string | null;
  /** `redo`: two inbox models linked to their sends, and both keys saved. */
  inbox?: string | null;
  /** `named` or `unmatched`: a download for the Downloads watcher. */
  arrival?: string | null;
  /** `demo`: two built assets on Frostveil, one current and one stale. */
  built?: string | null;
  /** `demo`, `rest`, `over` or `still`: models that move (see create-motion-handlers.ts). */
  motion?: string | null;
  /** `demo`: a frost lantern split into parts and a rune chest (see create-parts-handlers.ts). */
  parts?: string | null;
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
  /** `many`: Frostveil gets 49 more versioned assets, so the Inspector's compare menu is long. */
  inspect?: string | null;
  /** `hand-built`: Frostveil ships as a hand-built site (see create-ship-handlers.ts). */
  ship?: string | null;
  /** `many`: Frostveil's runner lists 24 models that all skip the bake (see pipeline-many.ts). */
  pipeline?: string | null;
  /** `held`, `repo` or `conflict`: a workspace shared with another computer (see create-sync-handlers.ts). */
  sync?: string | null;
  /** `available`, `ready`, `current` or `pending`: the updater is on (see create-app-handlers.ts). */
  updater?: string | null;
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
