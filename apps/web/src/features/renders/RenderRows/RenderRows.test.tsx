import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { RenderRecord } from '@/ipc/client';
import { useRenderStore } from '@/state/use-render-store';
import RenderRows from './RenderRows';

const target = (key: string) => ({
  key,
  name: key,
  folder: key,
  version: 1,
  file: `/m/${key}.glb`,
  thumbnail: null,
  heightM: null,
  triangles: 10,
  hasColour: true,
  unreadable: null,
});
const file = (style: 'clay' | 'toon', state: 'done' | 'running' | 'queued') => ({
  style,
  file: `/o/${style}.png`,
  state,
  bytes: null,
  seconds: null,
  problem: null,
  moving: null,
});

afterEach(() => useRenderStore.setState({ record: null }));

describe('the rows of a render', () => {
  it('says where each asset stands', () => {
    useRenderStore.setState({
      record: {
        items: [
          { target: target('Lantern'), files: [file('clay', 'done'), file('toon', 'done')] },
          { target: target('Gift box'), files: [file('clay', 'done'), file('toon', 'running')] },
          { target: target('Snowman'), files: [file('clay', 'queued')] },
        ],
      } as unknown as RenderRecord,
    });
    render(<RenderRows />);
    expect(screen.getByText('2 files')).toBeVisible();
    expect(screen.getByText('toon 2 of 2')).toBeVisible();
    expect(screen.getByText('queued')).toBeVisible();
  });
});
