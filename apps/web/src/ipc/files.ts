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

/**
 * The URL for `path`. A `stamp` (such as when the file was written) makes
 * the app's URL new each time a file is written over, so the web view does
 * not show a cached copy.
 */
export function fileUrl(path: string | null | undefined, stamp?: string): string | null {
  if (!path) return null;
  if (IN_TAURI) {
    const url = convertFileSrc(path);
    return stamp ? `${url}?v=${encodeURIComponent(stamp)}` : url;
  }
  return mockFiles.get(path) ?? null;
}

/**
 * Where a page frame loads `address`: as it is in the app; mock mode
 * serves its own page for the mock dev server's address.
 */
export function frameUrl(address: string | null): string | null {
  if (!address || IN_TAURI) return address;
  try {
    const url = new URL(address);
    const page = mockFiles.get(`${url.origin}/`);
    if (!page) return address;
    return url.searchParams.has('maquette-poster') ? `${page}#maquette-poster` : page;
  } catch {
    return address;
  }
}
