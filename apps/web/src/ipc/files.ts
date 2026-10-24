/**
 * URLs for files on disk. In the app they go through Tauri's asset
 * protocol; mock mode maps a few known paths to bundled files.
 */
import { convertFileSrc } from '@tauri-apps/api/core';
import { IN_TAURI } from './runtime';

const mockFiles = new Map<string, string>();

/** Mock mode only: serve `url` for `path`. */
export function registerMockFile(path: string, url: string): void {
  mockFiles.set(path, url);
}

export function fileUrl(path: string | null | undefined): string | null {
  if (!path) return null;
  if (IN_TAURI) return convertFileSrc(path);
  return mockFiles.get(path) ?? null;
}
