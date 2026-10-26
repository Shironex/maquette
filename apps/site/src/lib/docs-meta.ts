import { execFileSync } from 'node:child_process';
import tauri from '../../../desktop/src-tauri/tauri.conf.json';

/** The desktop app's version, which every docs page says it applies to. */
export const APP_VERSION: string = tauri.version;

const git = (args: string[]): string => {
  try {
    return execFileSync('git', args, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return '';
  }
};

/** A shallow clone dates every file with the checkout, which would be wrong. */
const shallow = (): boolean => git(['rev-parse', '--is-shallow-repository']) !== 'false';

const dates = new Map<string, Date | undefined>();

/**
 * The date of the last commit that touched `file`. Nothing when git is not
 * there, the history is shallow or the file is not committed yet; the page then
 * leaves the date out.
 */
export function lastUpdated(file: string | undefined): Date | undefined {
  if (!file) return undefined;
  if (!dates.has(file)) {
    const stamp = shallow() ? '' : git(['log', '-1', '--format=%cI', '--', file]);
    const date = stamp ? new Date(stamp) : undefined;
    dates.set(file, date && !Number.isNaN(date.getTime()) ? date : undefined);
  }
  return dates.get(file);
}

/** "26 Oct 2026", the same wherever the build runs. */
export function formatDate(date: Date): string {
  return date.toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

/** "Maquette 0.1.0 · Last updated 26 Oct 2026", or the version alone. */
export function versionLine(date: Date | undefined): string {
  const version = `Maquette ${APP_VERSION}`;
  return date ? `${version} · Last updated ${formatDate(date)}` : version;
}
