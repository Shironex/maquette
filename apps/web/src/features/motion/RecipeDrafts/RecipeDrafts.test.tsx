import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { RecipeDrafts as Drafts } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeDrafts from './RecipeDrafts';

afterEach(clearMocks);

/** Lays `drafts` on the open recipe as a read from the core would carry them. */
function withDrafts(drafts: Drafts | undefined) {
  const editing = useRecipeEditorStore.getState().editing!;
  useRecipeEditorStore.setState({ editing: { ...editing, doc: { ...editing.doc, drafts } } });
}

describe('the agent drafts banner', () => {
  it('says what the agent drafted until a person saves, with the legend', async () => {
    await openRecipe('slash-with-a-lean');
    const { container, rerender } = render(<RecipeDrafts />);
    expect(container).toBeEmptyDOMElement();
    withDrafts({
      by: 'Motion agent',
      at: clock.now().toISOString(),
      writes: 1,
      layers: ['l-1'],
      keys: [{ layer: 'l-1', clip: 0, frame: 20 }],
    });
    rerender(<RecipeDrafts />);
    const banner = screen.getByRole('status', { name: 'Agent drafts' });
    expect(banner).toHaveTextContent('Motion agent changed this recipe just now');
    expect(banner).toHaveTextContent('Drafted: 1 key and 1 change.');
    expect(banner).toHaveTextContent('Saving keeps them as yours.');
    expect(banner).toHaveTextContent('Blue: drafted by an agent, until you save.');

    // A person's save comes back without marks.
    await useRecipeEditorStore.getState().save();
    expect(useRecipeEditorStore.getState().editing?.doc.drafts).toBeUndefined();
    expect(screen.queryByRole('status', { name: 'Agent drafts' })).toBeNull();
  });
});
