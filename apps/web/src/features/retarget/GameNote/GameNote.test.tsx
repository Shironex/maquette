import { render } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import GameNote from './GameNote';

beforeEach(() => {
  useAppStore.setState({ workspace: null });
  useMotionStore.setState({ view: null, draft: undefined });
});

describe('the budget note', () => {
  it('says nothing before the project and its motion are read', () => {
    const { container } = render(<GameNote asset="a" name="Clay figure" adds={1000} />);
    expect(container).toBeEmptyDOMElement();
  });
});
