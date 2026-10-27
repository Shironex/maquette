import { render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { pictureOf } from '@/state/library';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryPicture from './LibraryPicture';

afterEach(() => clearMocks());

describe('a library picture', () => {
  it('shows the newest render, or the kind icon without one', async () => {
    await openMockLibrary('demo');
    const { items } = await commands.libraryGet();
    const island = items.find(i => i.entry.id === 'veil-platform')!;
    const found = items.find(i => i.entry.id === 'crypt-brazier')!;
    const bare = found.entry.versions.map(v => ({ ...v, thumbnail: null }));
    const brazier = { ...found, entry: { ...found.entry, versions: bare, refs: [] } };
    registerMockFile(pictureOf(island)!, '/mock/veil-platform.webp');
    const { container, rerender } = render(<LibraryPicture item={island} />);
    expect(container.querySelector('img')).not.toBeNull();
    rerender(<LibraryPicture item={brazier} />);
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('svg')).not.toBeNull();
  });
});
