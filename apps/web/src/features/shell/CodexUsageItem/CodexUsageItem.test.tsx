import { clock } from '@/lib/clock';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { mockUsage } from '@/ipc/mock/create-codex-usage-handlers';
import { resetTime } from '@/state/codex-usage';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';
import { CodexUsageItem } from '.';

const store = useCodexUsageStore.getState();
afterEach(() => useCodexUsageStore.setState(store, true));

describe("the status bar's Codex usage", () => {
  it('shows both windows and, on hover, when each resets', async () => {
    const now = clock.nowMs();
    const usage = mockUsage('ok', now);
    render(<CodexUsageItem usage={usage} />);
    const button = screen.getByRole('button', { name: 'Codex 16% (5h) · 3% (week)' });
    expect(button).toHaveAttribute('data-tone', 'ok');
    await userEvent.hover(button);
    const tip = await screen.findByRole('tooltip');
    expect(tip).toHaveTextContent('Codex usage, click to read again');
    expect(tip).toHaveTextContent(
      `5h window: 16% used, resets ${resetTime(usage.primary!.resetsAtMs!, now)}`
    );
    expect(within(tip).getByText(/Weekly: 3% used, resets \w{3} \d\d:\d\d/)).toBeVisible();
  });

  it('turns amber from 80% and red at the limit', () => {
    const { rerender } = render(<CodexUsageItem usage={mockUsage('low', clock.nowMs())} />);
    expect(screen.getByRole('button', { name: 'Codex 86% (5h) · 41% (week)' })).toHaveAttribute(
      'data-tone',
      'warn'
    );
    rerender(<CodexUsageItem usage={mockUsage('limit', clock.nowMs())} />);
    const limited = screen.getByRole('button', { name: 'Codex 100% (5h) · 44% (week)' });
    expect(limited).toHaveAttribute('data-tone', 'limit');
    expect(limited).toHaveClass('text-danger');
  });

  it('reads again on click', async () => {
    const read = vi.fn(async () => {});
    useCodexUsageStore.setState({ read });
    render(<CodexUsageItem usage={mockUsage('unknown', clock.nowMs())} />);
    await userEvent.click(screen.getByRole('button', { name: 'Codex usage unknown' }));
    expect(read).toHaveBeenCalledTimes(1);
  });
});
