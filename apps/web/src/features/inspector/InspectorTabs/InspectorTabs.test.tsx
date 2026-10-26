import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useInspectorStore } from '@/state/use-inspector-store';
import InspectorTabs from './InspectorTabs';

afterEach(() => useInspectorStore.setState({ tab: 'compare' }));

describe('the Inspector tabs', () => {
  it('opens the layout tab and back', async () => {
    render(<InspectorTabs />);
    expect(screen.getByRole('tab', { name: 'Compare' })).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(screen.getByRole('tab', { name: 'Layout' }));
    expect(useInspectorStore.getState().tab).toBe('layout');
    expect(screen.getByRole('tab', { name: 'Layout' })).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(screen.getByRole('tab', { name: 'Paths' }));
    expect(useInspectorStore.getState().tab).toBe('paths');
  });
});
