import type {
  Arrival,
  ConceptEvent,
  ErrorPayload,
  McpStatus,
  PipelineEvent,
  ProjectManifest,
  ProviderEvent,
} from '../client';
import { HOME } from './fixtures';

export type Args = Record<string, unknown>;

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
