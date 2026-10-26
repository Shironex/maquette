import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openMotion } from '@/test/open-motion';
import StatesPanel from './StatesPanel';

afterEach(() => clearMocks());

describe('the states panel', () => {
  it('says where states are kept and which triggers the site plays', async () => {
    await openMotion(undefined, 'switch-cabinet');
    render(<StatesPanel />);
    expect(screen.getByText('layout.json')).toBeInTheDocument();
    expect(screen.getByText('Stop and scroll triggers')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Transition' })).toBeInTheDocument();
  });

  it('asks for states on a model with none', async () => {
    await openMotion(undefined, 'turntable');
    render(<StatesPanel />);
    expect(screen.getByText(/Give the model states to start/)).toBeInTheDocument();
  });
});
