/**
 * Settings > About: the project's links, the version block made for
 * pasting into a bug report, and the address of the bug report form with
 * its fields filled in. Nothing is sent anywhere; the user opens and pastes.
 */
import type { AboutInfo, HostOs, UpdaterMode } from '@/ipc/client';

export const REPO_URL = 'https://github.com/Shironex/maquette';
export const DOCS_URL = 'https://shironex.github.io/maquette/';
/** The bug report issue form, `.github/ISSUE_TEMPLATE/bug_report.yml`. */
export const BUG_FORM = 'bug_report.yml';

const OS_NAME: Record<HostOs, string> = { mac: 'macOS', windows: 'Windows', linux: 'Linux' };

/** True for a development build, which has no commit or date stamped in. */
export function isDevBuild(about: AboutInfo): boolean {
  return about.commit === 'dev';
}

/** `macOS 15.2`, or the name alone when the OS did not say its version. */
export function osName(about: AboutInfo): string {
  const name = OS_NAME[about.os];
  return about.osVersion ? `${name} ${about.osVersion}` : name;
}

/** `macOS 15.2 (aarch64)`. */
export function osLine(about: AboutInfo): string {
  return `${osName(about)} (${about.arch})`;
}

/** `0.1.0 (a1b2c3d4e5)`, or `0.1.0 (dev build)`. */
export function versionLine(about: AboutInfo): string {
  return `${about.version} (${isDevBuild(about) ? 'dev build' : about.commit})`;
}

const UPDATES_LINE: Record<UpdaterMode, string> = {
  on: 'on',
  unreleased: 'off until the first public release',
  'dev-build': 'off in a development build',
  'turned-off': 'turned off for this run',
};

/** The Blender version to report: the shell's, else the toolchain check's. */
export function blenderVersion(about: AboutInfo, checked?: string | null): string | null {
  return about.blender ?? checked ?? null;
}

/** The block "Copy version info" puts on the clipboard. */
export function versionBlock(about: AboutInfo, blender: string | null): string {
  const built = isDevBuild(about) ? 'dev build' : `built ${about.buildDate}`;
  return [
    `${about.name} ${about.version} (${isDevBuild(about) ? 'dev' : `commit ${about.commit}`}, ${built})`,
    `OS: ${osLine(about)}`,
    `Tauri: ${about.tauriVersion}`,
    `Blender: ${blender ?? 'not checked'}`,
    `Updates: ${UPDATES_LINE[about.updater.mode]}`,
  ].join('\n');
}

/**
 * The bug report form with its text fields filled: `version`, `os-version`
 * and `blender`, the ids in `bug_report.yml`. GitHub fills only text fields
 * from the address, so the OS and "How you run it" dropdowns stay for the
 * user to pick.
 */
export function reportUrl(about: AboutInfo, blender: string | null): string {
  const params = new URLSearchParams({ template: BUG_FORM });
  params.set('version', versionLine(about));
  params.set('os-version', osLine(about));
  if (blender) params.set('blender', blender);
  return `${REPO_URL}/issues/new?${params.toString()}`;
}

/** The release notes of this version, or every release for a dev build. */
export function releaseNotesUrl(about: AboutInfo): string {
  return isDevBuild(about)
    ? `${REPO_URL}/releases`
    : `${REPO_URL}/releases/tag/v${encodeURIComponent(about.version)}`;
}
