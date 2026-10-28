import { clock } from '@/lib/clock';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LabView } from '@/ipc/client';
import { mockUsage } from '@/ipc/mock/create-codex-usage-handlers';
import { runningJobs } from '@/ipc/mock/lab-fixtures';
import { EMPTY_STYLE } from '@/state/lab-style';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';
import { UsageLimitBanner } from '.';

const store = useCodexUsageStore.getState();
afterEach(() => useCodexUsageStore.setState(store, true));

function view(limit?: 'usage' | 'rate'): LabView {
  return {
    project: 'p',
    projectDir: '/w/p',
    directions: [],
    jobs: runningJobs('p', clock.nowMs(), limit),
    refs: [],
    concurrency: 4,
    style: EMPTY_STYLE,
  };
}

describe('the usage limit banner', () => {
  it('stays away while no job waits for the usage limit', () => {
    const { container } = render(<UsageLimitBanner view={view('rate')} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("says when the limit lifts by Codex's reset time and retries on request", async () => {
    const usage = mockUsage('limit', clock.nowMs());
    const lift = vi.fn(async () => null);
    useCodexUsageStore.setState({ usage, lift });
    render(<UsageLimitBanner view={view('usage')} />);
    const banner = screen.getByRole('status');
    expect(banner).toHaveTextContent('Codex usage limit reached');
    expect(banner).toHaveTextContent(
      /Codex says it lifts at about (\w{3} )?\d\d:\d\d\. 3 images wait and run again on their own then\./
    );
    await userEvent.click(screen.getByRole('button', { name: 'Limit lifted, retry now' }));
    expect(lift).toHaveBeenCalledTimes(1);
  });

  it('shows the answer when the limit still holds', () => {
    const usage = mockUsage('limit', clock.nowMs());
    useCodexUsageStore.setState({
      usage,
      lifted: {
        usage,
        lifted: false,
        jobs: [],
        message: 'The usage limit still holds until about 16:20.',
      },
    });
    render(<UsageLimitBanner view={view('usage')} />);
    expect(screen.getByText('The usage limit still holds until about 16:20.')).toBeVisible();
  });
});
