import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../openProviders';
import { HandoffView } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [] });
});

describe('the by hand settings card', () => {
  it('walks through the four steps for a waiting send', async () => {
    const { job } = await openProviders();
    const lanterns = job('lanterns-1');
    render(<HandoffView job={lanterns} />);
    expect(screen.getByText('Waiting for your model')).toBeInTheDocument();
    const steps = screen.getAllByRole('listitem').filter(li => li.parentElement?.tagName === 'OL');
    expect(steps.map(li => li.querySelector('div.font-semibold')?.textContent)).toEqual([
      'Open Tripo',
      'Upload the images',
      'Use these settings',
      'Bring the GLB back',
    ]);
    expect(screen.getByText(lanterns.card!.returnName)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add model...' })).toBeInTheDocument();
  });

  it('opens the web app, the folder and an image through the backend', async () => {
    const { job, sent } = await openProviders();
    const lanterns = job('lanterns-1');
    render(<HandoffView job={lanterns} />);
    await userEvent.click(screen.getByRole('button', { name: 'Open Tripo studio' }));
    await userEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    const images = screen.getByText(lanterns.card!.images[0]!).closest('li')!;
    await userEvent.click(within(images).getByRole('button', { name: 'Reveal image' }));
    expect(sent('provider_open_site')).toHaveLength(1);
    expect(sent('provider_open_handoff')).toHaveLength(1);
    expect(sent('provider_reveal_image')).toHaveLength(1);
  });

  it('renders nothing for a send without a card', async () => {
    const { job } = await openProviders();
    const { container } = render(<HandoffView job={job('sakura-1')} />);
    expect(container).toBeEmptyDOMElement();
  });
});
