import type { DownloadMatch, DownloadsScan, ProviderJob } from '../client';
import { clock } from '@/lib/clock';
import { HOME } from './fixtures';

/** The fake Downloads folder: files that look like the models By hand sends wait for. */
export function scanDownloads(
  jobs: ProviderJob[],
  project: unknown,
  fallbackMs: number
): DownloadsScan {
  const waiting = jobs.filter(
    j => j.project === project && j.provider === 'by-hand' && j.state === 'waiting'
  );
  const since = waiting.length ? Math.min(...waiting.map(j => j.createdAtMs ?? fallbackMs)) : null;
  const at = clock.nowMs();
  const file = (
    name: string,
    inner: string | null,
    bytes: number,
    ago: number,
    job: ProviderJob | undefined,
    reason: string | null
  ): DownloadMatch => ({
    path: `${HOME}/Downloads/${name}`,
    name,
    inner,
    bytes,
    modifiedMs: at - ago * 60_000,
    job: job?.id ?? null,
    assetName: job?.assetName ?? null,
    reason: job ? reason : null,
  });
  const [first, second] = waiting;
  const slug = (j: ProviderJob) => j.assetName.toLowerCase().replace(/[^a-z0-9]+/g, '_');
  const files = waiting.length
    ? [
        ...(first
          ? [file(`${slug(first)}.glb`, null, 4.2e6, 3, first, `Named like ${first.assetName}`)]
          : []),
        ...(second
          ? [
              file(
                `tripo_${slug(second)}_8c1f.zip`,
                `${slug(second)}.glb`,
                6.8e6,
                7,
                second,
                `Name close to ${second.assetName}`
              ),
            ]
          : []),
        file('model (3).glb', null, 3.1e6, 12, undefined, null),
      ]
    : [];
  return { folder: `${HOME}/Downloads`, sinceMs: since, files, waiting };
}
