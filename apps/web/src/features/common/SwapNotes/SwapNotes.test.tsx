import { render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import SwapNotes from './SwapNotes';

afterEach(() => clearMocks());

const open = (built: string) =>
  openMockWorkspace('configured', undefined, undefined, undefined, undefined, built);

describe('SwapNotes', () => {
  it('names each model a built asset replaced, with both files and their numbers', async () => {
    await open('swapped');
    render(<SwapNotes project="frostveil" />);
    const notes = await screen.findByRole('region', {
      name: 'Built models that replace another model in this Ship',
    });
    expect(within(notes).getByRole('heading')).toHaveTextContent('Replaced by a built model 1');
    expect(within(notes).getByRole('listitem')).toHaveTextContent(
      'Crystal lantern is a built model now: crystal_lantern_v3.glb takes the place of crystal_lantern_v2.glb · 1.8k triangles, was 12.9k · 96 KB, was 400 KB'
    );
  });

  it('draws nothing while no swap waits for a Ship', async () => {
    await open('swap');
    const { container } = render(<SwapNotes project="frostveil" />);
    // The read settles with no lines.
    await new Promise(resolve => setTimeout(resolve, 20));
    expect(container).toBeEmptyDOMElement();
  });
});
