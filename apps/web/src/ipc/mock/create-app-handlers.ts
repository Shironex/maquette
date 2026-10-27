/**
 * The app's own commands in mock mode: quitting (nothing closes in the
 * browser, so those do nothing), Settings > About and the app updater. By default the updater
 * is inert, as it is until the first public release. `?updater=` shows it on:
 *
 * - `available`: a check found 0.2.0 with release notes; Download streams
 *   progress, then Restart to update waits.
 * - `ready`: 0.2.0 is downloaded and waits for a restart.
 * - `current`: on, and the newest version.
 * - `pending`: on, and no release is out for this computer yet.
 *
 * A restart never happens in the browser; it is refused like one asked
 * while a run is going.
 */
import type { AboutInfo, UpdateInfo, UpdaterEvent, UpdaterStatus } from '../client';
import { fail, sleep, type Args } from './backend-options';

export const MOCK_UPDATE: UpdateInfo = {
  version: '0.2.0',
  notes: [
    'Highlights',
    '- Ship to site sends a finished scene straight to the site repo.',
    '- Built assets rebuild before a run when their script changed.',
    '',
    'Bug fixes',
    '- The Inbox keeps its preview mode after a reprocess.',
    '- Long Lab titles no longer push the Send button off the card.',
  ].join('\n'),
  date: '2026-11-03',
};

export const MOCK_ENDPOINT =
  'https://github.com/Shironex/maquette/releases/latest/download/latest.json';

export function createAppHandlers({
  state,
  emit,
  stepMs = 220,
}: {
  /** `?updater=`, see above. */
  state: string | null;
  emit?: (payload: UpdaterEvent) => void;
  stepMs?: number;
}): Record<string, (args: Args) => unknown> {
  const on = state !== null && ['available', 'ready', 'current', 'pending'].includes(state);
  let ready = state === 'ready';
  const found: UpdateInfo | null = state === 'available' || state === 'ready' ? MOCK_UPDATE : null;

  const status = (): UpdaterStatus => ({
    mode: on ? 'on' : 'unreleased',
    endpoint: MOCK_ENDPOINT,
    available: found,
    ready,
  });

  const about = (): AboutInfo => ({
    name: 'Maquette',
    version: '0.1.0',
    commit: '4137061f2a',
    buildDate: '2026-10-27',
    os: 'mac',
    osVersion: '15.2',
    arch: 'aarch64',
    tauriVersion: '2.11.5',
    // As before any toolchain check; About then reads the check's version.
    blender: null,
    updater: status(),
  });

  const send = (event: UpdaterEvent) => emit?.(event);

  return {
    quit_set_unapplied: () => undefined,
    quit_now: () => undefined,
    app_about: () => about(),
    app_open_link: () => undefined,
    updater_status: () => status(),
    updater_check: async (): Promise<UpdaterEvent> => {
      if (!on) fail('invalid', 'Updates start with the first public release.');
      send({ type: 'checking' });
      await sleep(stepMs);
      const event: UpdaterEvent = ready
        ? { type: 'ready', update: MOCK_UPDATE }
        : state === 'pending'
          ? { type: 'pending' }
          : found
            ? { type: 'available', update: found }
            : { type: 'current' };
      send(event);
      return event;
    },
    updater_download: async () => {
      if (!found) fail('invalid', 'There is no update to download. Check for updates first.');
      const total = 48_000_000;
      for (let i = 1; i <= 4; i++) {
        await sleep(stepMs);
        send({ type: 'progress', downloaded: (total * i) / 4, total });
      }
      ready = true;
      send({ type: 'ready', update: found });
      return found;
    },
    updater_install: () =>
      fail(
        'invalid',
        'A pipeline run or a build is still going. Let it finish or cancel it, then restart.'
      ),
  };
}
