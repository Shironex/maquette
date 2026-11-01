import { registerMockFile } from '@/ipc/files';

/** The images Maquette takes: the core checks each file's header too. */
export const IMAGE_FILE = /\.(png|jpe?g|webp)$/i;

/**
 * Paths for image files dropped in a plain browser (mock mode), which gets
 * File objects without paths. The app gets paths from `useNativeDrop`.
 */
export function mockDroppedImages(files: Iterable<File> | ArrayLike<File>): string[] {
  const paths: string[] = [];
  for (const file of Array.from(files)) {
    if (!IMAGE_FILE.test(file.name)) continue;
    const path = `/dropped/${file.name}`;
    registerMockFile(path, URL.createObjectURL(file));
    paths.push(path);
  }
  return paths;
}
