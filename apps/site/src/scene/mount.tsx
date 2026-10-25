/**
 * The 3D island's entry. The page imports it on demand (desktop class
 * devices with WebGL 2, after first paint), so phones never download it.
 */
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import type { PosterCamera, StopId } from './manifest';
import { FAILED_EVENT, READY_EVENT, type TourScroll } from './tour';
import { Workbench } from './Workbench';

export interface MountOptions {
  initialStop: StopId;
  scroll: TourScroll;
  capture?: PosterCamera | 'stops';
}

const ready = () => window.dispatchEvent(new Event(READY_EVENT));
const failed = (reason: string) =>
  window.dispatchEvent(new CustomEvent(FAILED_EVENT, { detail: { reason } }));

export function mount(container: HTMLElement, options: MountOptions): () => void {
  const root = createRoot(container);
  root.render(
    <StrictMode>
      <Workbench
        initialStop={options.initialStop}
        scroll={options.scroll}
        onReady={ready}
        onFailed={failed}
        capture={options.capture}
      />
    </StrictMode>
  );
  return () => root.unmount();
}
