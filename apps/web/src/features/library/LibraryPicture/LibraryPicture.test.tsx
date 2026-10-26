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
    const island = items.find(i => i.entry.id === 'island')!;
    const brazier = items.find(i => i.entry.id === 'crypt-brazier')!;
    registerMockFile(pictureOf(island)!, '/mock/island.webp');
    const { container, rerender } = render(<LibraryPicture item={island} />);
    expect(container.querySelector('img')).not.toBeNull();
    rerender(<LibraryPicture item={brazier} />);
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('svg')).not.toBeNull();
  });
});
