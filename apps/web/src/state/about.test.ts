import { describe, expect, it } from 'vitest';
import type { AboutInfo } from '@/ipc/client';
import { blenderVersion, osLine, releaseNotesUrl, reportUrl, versionBlock } from './about';

const about: AboutInfo = {
  name: 'Maquette',
  version: '0.1.0',
  commit: 'a1b2c3d4e5',
  buildDate: '2026-10-27',
  os: 'mac',
  osVersion: '15.2',
  arch: 'aarch64',
  tauriVersion: '2.11.5',
  blender: null,
  updater: { mode: 'unreleased', endpoint: null, available: null, ready: false },
};
const dev: AboutInfo = {
  ...about,
  commit: 'dev',
  buildDate: 'dev',
  os: 'windows',
  osVersion: null,
};

describe('the version block', () => {
  it('carries what a bug report needs', () => {
    expect(versionBlock(about, '5.0.1')).toBe(
      [
        'Maquette 0.1.0 (commit a1b2c3d4e5, built 2026-10-27)',
        'OS: macOS 15.2 (aarch64)',
        'Tauri: 2.11.5',
        'Blender: 5.0.1',
        'Updates: off until the first public release',
      ].join('\n')
    );
  });

  it('names a dev build and a missing Blender check', () => {
    const block = versionBlock(dev, null);
    expect(block).toContain('Maquette 0.1.0 (dev, dev build)');
    expect(block).toContain('OS: Windows (aarch64)');
    expect(block).toContain('Blender: not checked');
  });

  it('prefers the shell Blender version over the toolchain check', () => {
    expect(blenderVersion({ ...about, blender: '4.4' }, '5.0.1')).toBe('4.4');
    expect(blenderVersion(about, '5.0.1')).toBe('5.0.1');
    expect(blenderVersion(about)).toBeNull();
    expect(osLine(dev)).toBe('Windows (aarch64)');
  });
});

describe('the report address', () => {
  it('opens the bug form with the text fields of bug_report.yml filled', () => {
    const url = new URL(reportUrl(about, '5.0.1'));
    expect(`${url.origin}${url.pathname}`).toBe('https://github.com/Shironex/maquette/issues/new');
    expect(url.searchParams.get('template')).toBe('bug_report.yml');
    expect(url.searchParams.get('version')).toBe('0.1.0 (a1b2c3d4e5)');
    expect(url.searchParams.get('os-version')).toBe('macOS 15.2 (aarch64)');
    expect(url.searchParams.get('blender')).toBe('5.0.1');
  });

  it('leaves Blender empty when it is unknown and says dev build', () => {
    const url = new URL(reportUrl(dev, null));
    expect(url.searchParams.has('blender')).toBe(false);
    expect(url.searchParams.get('version')).toBe('0.1.0 (dev build)');
  });

  it('uses the field ids the form declares', async () => {
    const { readFileSync } = await import('node:fs');
    const { resolve } = await import('node:path');
    const form = readFileSync(
      resolve(import.meta.dirname, '../../../../.github/ISSUE_TEMPLATE/bug_report.yml'),
      'utf8'
    );
    for (const key of new URL(reportUrl(about, '5.0.1')).searchParams.keys()) {
      if (key === 'template') continue;
      // Only text inputs fill from the address.
      expect(form).toMatch(new RegExp(`type: input\\n\\s+id: ${key}\\n`));
    }
  });

  it('links the release notes of this version', () => {
    expect(releaseNotesUrl(about)).toBe('https://github.com/Shironex/maquette/releases/tag/v0.1.0');
    expect(releaseNotesUrl(dev)).toBe('https://github.com/Shironex/maquette/releases');
  });
});
