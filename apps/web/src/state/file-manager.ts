import type { HostOs } from '@/ipc/client';

/** What the OS calls its file manager. Before boot it reads as a Mac. */
export function fileManagerName(os: HostOs | undefined): string {
  return os === 'windows' ? 'Explorer' : os === 'linux' ? 'the file manager' : 'Finder';
}

/**
 * An action that shows `what` selected in the file manager, in the OS's
 * words: "Reveal image in Finder", "Show image in Explorer".
 */
export function revealLabel(os: HostOs | undefined, what: string): string {
  const verb = os === 'windows' || os === 'linux' ? 'Show' : 'Reveal';
  return `${verb} ${what} in ${fileManagerName(os)}`;
}
