import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { Monogram } from '../brand/Monogram';

/**
 * Who draws the window controls:
 * - `drawn`: nobody else does (a plain browser), so the kit draws the lights.
 * - `overlay`: macOS draws native lights over the web view; the kit keeps their space free.
 * - `native`: the OS title bar sits above the web view; the kit draws nothing.
 */
export type ChromeMode = 'drawn' | 'overlay' | 'native';

export function TrafficLights({ mode, className }: { mode: ChromeMode; className?: string }) {
  if (mode === 'native') return null;
  const lights = ['#ff5f57', '#febc2e', '#28c840'].map(c => (
    <i
      key={c}
      className="block size-3 rounded-full shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.25)]"
      style={{ background: c }}
    />
  ));
  return (
    <div
      aria-hidden="true"
      data-tauri-drag-region
      className={cn('flex h-3.5 items-center gap-2', className)}
    >
      {mode === 'drawn' ? lights : <span className="w-[52px]" />}
    </div>
  );
}

export interface WindowChromeProps {
  title?: string;
  mode: ChromeMode;
  /** Shows the monogram before the title, as the app window does. */
  mark?: boolean;
  className?: string;
  children?: ReactNode;
}

/** The 36 px title strip with the lights and a centred title, then the window body. */
export function WindowChrome({
  title = 'Maquette',
  mode,
  mark,
  className,
  children,
}: WindowChromeProps) {
  return (
    <div className={cn('flex h-full min-h-0 flex-col bg-app text-fg', className)}>
      {mode === 'native' ? null : (
        <div
          data-tauri-drag-region
          className={cn(
            'relative flex flex-none items-center border-b border-line bg-panel px-4',
            // The native lights sit 23px down in overlay mode, so the bar centres on them.
            mode === 'overlay' ? 'h-[46px]' : 'h-9'
          )}
        >
          <TrafficLights mode={mode} />
          <span className="pointer-events-none absolute inset-x-0 flex items-center justify-center gap-2 text-[12.5px] font-medium text-fg-dim">
            {mark ? <Monogram size={16} /> : null}
            {title}
          </span>
        </div>
      )}
      <div className="flex min-h-0 flex-1">{children}</div>
    </div>
  );
}
