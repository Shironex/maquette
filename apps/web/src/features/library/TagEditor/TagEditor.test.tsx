import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import TagEditor from './TagEditor';

afterEach(() => clearMocks());

const entryOf = async (id: string) =>
  (await commands.libraryGet()).items.find(i => i.entry.id === id)!.entry;

describe('a model tag editor', () => {
  it('adds a tag with Enter and takes one off with its x', async () => {
    await openMockLibrary('demo');
    const entry = await entryOf('veil-platform');
    const { rerender } = render(<TagEditor entry={entry} />);
    await userEvent.type(
      screen.getByRole('textbox', { name: 'Add a tag to Veil platform' }),
      'Night{Enter}'
    );
    await waitFor(async () => expect((await entryOf('veil-platform')).tags).toContain('night'));
    rerender(<TagEditor entry={await entryOf('veil-platform')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Take the tag night off' }));
    await waitFor(async () => expect((await entryOf('veil-platform')).tags).not.toContain('night'));
  });
});
