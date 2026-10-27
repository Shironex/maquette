import type { HostOs } from '@/ipc/client';

/** What the OS calls its file manager. Before boot it reads as a Mac. */
export function fileManagerName(os: HostOs | undefined): string {
  return os === 'windows' ? 'Explorer' : os === 'linux' ? 'the file manager' : 'Finder';
}

/** The verb for showing a file in the file manager: "Reveal" on a Mac, else "Show". */
export function revealVerb(os: HostOs | undefined): string {
  return os === 'windows' || os === 'linux' ? 'Show' : 'Reveal';
}

/**
 * An action that shows `what` selected in the file manager, in the OS's
 * words: "Reveal image in Finder", "Show image in Explorer".
 */
export function revealLabel(os: HostOs | undefined, what: string): string {
  return `${revealVerb(os)} ${what} in ${fileManagerName(os)}`;
}
