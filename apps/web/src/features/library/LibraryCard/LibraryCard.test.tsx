import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryCard from './LibraryCard';

afterEach(() => clearMocks());

describe('a library card', () => {
  it('shows the version, source, tags, uses and what reuse saved', async () => {
    await openMockLibrary('demo');
    const item = (await commands.libraryGet()).items.find(i => i.entry.id === 'character')!;
    const onSelect = vi.fn();
    render(<LibraryCard item={item} selected={false} onSelect={onSelect} />);
    expect(screen.getByText('v3 · Meshy, rigged')).toBeVisible();
    expect(screen.getByText('characters')).toBeVisible();
    expect(screen.getByText('2 projects')).toBeVisible();
    expect(screen.getByText('40 saved by reuse')).toHaveClass('text-sage');
    await userEvent.click(screen.getByRole('button', { name: 'Character' }));
    expect(onSelect).toHaveBeenCalled();
  });
});
