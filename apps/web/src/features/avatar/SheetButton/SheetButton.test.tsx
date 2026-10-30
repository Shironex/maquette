import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SheetButton from './SheetButton';

describe('a sheet thumbnail', () => {
  it('shows the sheet under its label and opens it up close', async () => {
    const onOpen = vi.fn();
    render(<SheetButton src="after.webp" label="After" onOpen={onOpen} />);
    expect(screen.getByText('After')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'The after sheet' })).toHaveAttribute(
      'src',
      'after.webp'
    );
    await userEvent.click(screen.getByRole('button', { name: 'The after sheet' }));
    expect(onOpen).toHaveBeenCalledOnce();
  });

  it('keeps its place while the sheet is not there yet', () => {
    render(<SheetButton src={null} label="Before" onOpen={() => {}} />);
    expect(screen.queryByRole('img')).toBeNull();
    expect(screen.getByRole('button', { name: /before sheet up close/ })).toBeInTheDocument();
  });
});
