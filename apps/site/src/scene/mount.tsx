/**
 * The 3D island's entry. The page imports it on demand (desktop class
 * devices with WebGL 2, after first paint), so phones never download it.
 */
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import type { PosterCamera, StopId } from './manifest';
import { FAILED_EVENT, READY_EVENT, STOP_EVENT, type StopDetail } from './tour';
import { Workbench } from './Workbench';

export interface MountOptions {
  initialStop: StopId;
  reducedMotion: boolean;
  capture?: PosterCamera | 'stops';
}

function subscribe(listener: (stop: StopId) => void): () => void {
  const onStop = (event: Event) => listener((event as CustomEvent<StopDetail>).detail.id);
  window.addEventListener(STOP_EVENT, onStop);
  return () => window.removeEventListener(STOP_EVENT, onStop);
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
        reducedMotion={options.reducedMotion}
        subscribe={subscribe}
        onReady={ready}
        onFailed={failed}
        capture={options.capture}
      />
    </StrictMode>
  );
  return () => root.unmount();
}
