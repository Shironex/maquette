import { describe, expect, it } from 'vitest';
import type { UpdateInfo, UpdaterEvent } from '@/ipc/client';
import {
  INITIAL_UPDATER,
  downloadPercent,
  fromStatus,
  installing,
  phaseLine,
  reduce,
  showsPill,
  type UpdaterState,
} from './updater';

const UPDATE: UpdateInfo = { version: '0.2.0', notes: 'Faster bakes.', date: '2026-11-03' };
const on: UpdaterState = { mode: 'on', phase: { kind: 'idle' } };
const run = (state: UpdaterState, ...steps: UpdaterEvent[]) => steps.reduce(reduce, state);

describe('the updater state machine', () => {
  it('goes from a check through download to ready', () => {
    let s = run(on, { type: 'checking' });
    expect(s.phase.kind).toBe('checking');
    s = run(s, { type: 'available', update: UPDATE });
    expect(s.phase).toEqual({ kind: 'available', update: UPDATE });
    s = run(s, { type: 'progress', downloaded: 10, total: 40 });
    expect(s.phase).toMatchObject({ kind: 'downloading', downloaded: 10, total: 40 });
    expect(downloadPercent(s.phase)).toBe(25);
    s = run(s, { type: 'ready', update: UPDATE });
    expect(s.phase.kind).toBe('ready');
    expect(installing(s).phase).toEqual({ kind: 'installing', update: UPDATE });
  });

  it('keeps a download or a ready update when a background check reports', () => {
    const downloading = run(
      on,
      { type: 'available', update: UPDATE },
      {
        type: 'progress',
        downloaded: 1,
        total: null,
      }
    );
    for (const step of [{ type: 'checking' }, { type: 'current' }, { type: 'pending' }] as const) {
      expect(reduce(downloading, step)).toBe(downloading);
    }
    expect(downloadPercent(downloading.phase)).toBeUndefined();
    const ready = run(downloading, { type: 'ready', update: UPDATE });
    expect(reduce(ready, { type: 'available', update: UPDATE })).toBe(ready);
    expect(reduce(ready, { type: 'progress', downloaded: 5, total: 5 })).toBe(ready);
  });

  it('ignores progress with no update and keeps the update on a failure', () => {
    expect(reduce(on, { type: 'progress', downloaded: 1, total: 2 })).toBe(on);
    const failed = run(on, { type: 'available', update: UPDATE }, { type: 'failed', message: 'x' });
    expect(failed.phase).toEqual({ kind: 'failed', message: 'x', update: UPDATE });
  });

  it('starts from the status the shell gives', () => {
    expect(
      fromStatus({ mode: 'unreleased', endpoint: null, available: null, ready: false })
    ).toEqual({ mode: 'unreleased', phase: { kind: 'idle' } });
    expect(
      fromStatus({ mode: 'on', endpoint: null, available: UPDATE, ready: true }).phase.kind
    ).toBe('ready');
    expect(
      fromStatus({ mode: 'on', endpoint: null, available: UPDATE, ready: false }).phase.kind
    ).toBe('available');
  });

  it('says plainly why updates are off, and shows the pill only with an update', () => {
    expect(phaseLine({ mode: 'unreleased', phase: { kind: 'idle' } }, '0.1.0')).toMatch(
      /first public release/
    );
    expect(phaseLine({ mode: 'dev-build', phase: { kind: 'idle' } }, '0.1.0')).toMatch(
      /development build/
    );
    expect(phaseLine(INITIAL_UPDATER, '0.1.0')).toBe('');
    expect(phaseLine(run(on, { type: 'pending' }), '0.1.0')).toMatch(/No release is out/);
    expect(showsPill(on)).toBe(false);
    expect(showsPill(run(on, { type: 'available', update: UPDATE }))).toBe(true);
    expect(showsPill({ mode: 'unreleased', phase: { kind: 'available', update: UPDATE } })).toBe(
      false
    );
  });
});
