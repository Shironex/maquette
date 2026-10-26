import type { FrameState } from '@/state/page';

export interface FrameStatusProps {
  state: Exclude<FrameState, { kind: 'running' }>;
  /** The site's folder name, for the copy. */
  site: string;
  /** The command the server runs, before the port. */
  command: string | null;
}
