import { registerMockFile } from '@/ipc/files';

/** The composer's refs filter on this; a Concept column drop leaves it all to the core. */
export const IMAGE_FILE = /\.(png|jpe?g|webp)$/i;

/**
 * Paths for files dropped in a plain browser (mock mode), which gets
 * File objects without paths. The app gets paths from `useNativeDrop`.
 */
export function mockDroppedFiles(files: Iterable<File> | ArrayLike<File>): string[] {
  const paths: string[] = [];
  for (const file of Array.from(files)) {
    const path = `/dropped/${file.name}`;
    registerMockFile(path, URL.createObjectURL(file));
    paths.push(path);
  }
  return paths;
}
