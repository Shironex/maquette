import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { KeyRowView, useKeyDraft } from '..';

function Row({
  test,
}: {
  test: (key: string | null) => Promise<{ ok: boolean; message: string }>;
}) {
  const key = useKeyDraft({
    status: { saved: true, tail: 'k7Qm' },
    busy: false,
    save: async () => true,
    remove: async () => {},
    test,
  });
  return (
    <KeyRowView
      label="OpenAI"
      where="platform.openai.com, API keys"
      testTip="Lists the models"
      {...key}
    />
  );
}

describe('a key row view', () => {
  it('shows a saved key by its tail and tests the saved key', async () => {
    const test = vi.fn(async () => ({ ok: true, message: 'The key works.' }));
    render(<Row test={test} />);
    expect(screen.getByText(/^•{8}k7Qm$/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Test key' }));
    expect(test).toHaveBeenCalledWith(null);
    expect(await screen.findByRole('status')).toHaveTextContent('The key works.');
  });
});
