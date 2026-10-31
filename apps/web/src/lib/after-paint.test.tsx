import { Suspense } from 'react';
import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { afterPaint, lazyAfterPaint } from './after-paint';

afterEach(() => vi.restoreAllMocks());

describe('afterPaint', () => {
  it('waits for a frame, then for the task after it', async () => {
    const order: string[] = [];
    let frame: FrameRequestCallback = () => undefined;
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(callback => {
      frame = callback;
      return 1;
    });
    const done = afterPaint().then(() => order.push('resolved'));
    await Promise.resolve();
    order.push('before the frame');
    frame(0);
    order.push('the frame');
    await done;
    expect(order).toEqual(['before the frame', 'the frame', 'resolved']);
  });

  it('goes ahead when no frame ever comes, as in a hidden window', async () => {
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(() => 1);
    await expect(afterPaint()).resolves.toBeUndefined();
  });
});

describe('lazyAfterPaint', () => {
  const Page = ({ text }: { text: string }) => <p>{text}</p>;
  const load = () => Promise.resolve({ default: Page });
  const fallback = <p>Loading</p>;

  it('paints the fallback first, then the code', async () => {
    const Later = lazyAfterPaint(load);
    render(
      <Suspense fallback={fallback}>
        <Later text="The page" />
      </Suspense>
    );
    expect(screen.getByText('Loading')).toBeInTheDocument();
    expect(await screen.findByText('The page')).toBeInTheDocument();
  });

  it('shows no fallback for preloaded code', async () => {
    const Later = lazyAfterPaint(load);
    await Later.preload();
    render(
      <Suspense fallback={fallback}>
        <Later text="The page" />
      </Suspense>
    );
    expect(screen.queryByText('Loading')).toBeNull();
    expect(screen.getByText('The page')).toBeInTheDocument();
  });
});
