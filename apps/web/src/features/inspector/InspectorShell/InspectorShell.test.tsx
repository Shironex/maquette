import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import InspectorShell from './InspectorShell';

afterEach(() => clearMocks());

describe('the Inspector shell', () => {
  it("leads with the project's pipeline stages, then the page", async () => {
    await openMockWorkspace();
    render(
      <InspectorShell right={<button type="button">Run</button>}>
        <main>page</main>
      </InspectorShell>
    );
    expect(screen.queryByRole('heading', { name: 'Inspector' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Run' })).toBeInTheDocument();
    expect(screen.getByRole('main')).toHaveTextContent('page');
  });

  it('is titled Inspector without a project', () => {
    useAppStore.setState({ workspace: null });
    render(<InspectorShell>page</InspectorShell>);
    expect(screen.getByRole('heading', { name: 'Inspector' })).toBeInTheDocument();
  });
});
