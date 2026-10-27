import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryRow from './LibraryRow';

afterEach(() => clearMocks());

describe('a library row', () => {
  it('reads like the card, with its numbers', async () => {
    await openMockLibrary('demo');
    const item = (await commands.libraryGet()).items.find(i => i.entry.id === 'veil-platform')!;
    render(
      <ul>
        <LibraryRow item={item} selected onSelect={() => {}} />
      </ul>
    );
    expect(screen.getByRole('button', { name: 'Veil platform', pressed: true })).toBeVisible();
    expect(screen.getByText('v1 · Built')).toBeVisible();
    expect(screen.getByText('no credits, built')).toBeVisible();
    expect(screen.getByText('412k triangles · 1.1 MB')).toBeVisible();
  });
});
