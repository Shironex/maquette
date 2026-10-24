import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export interface LogLine {
  t: string;
  text: string;
  level?: 'info' | 'ok' | 'warn' | 'error';
}

export interface LogConsoleProps {
  title?: string;
  live?: boolean;
  height?: number;
  actions?: ReactNode;
  lines: readonly LogLine[];
  className?: string;
}

const LEVEL = {
  info: 'text-fg',
  ok: 'text-sage',
  warn: 'text-amber',
  error: 'text-danger',
} as const;

export function LogConsole({
  title = 'Log',
  live,
  height,
  actions,
  lines,
  className,
}: LogConsoleProps) {
  return (
    <div
      className={cn(
        'flex min-w-0 flex-col overflow-hidden rounded-card border border-line bg-app',
        className
      )}
    >
      <div className="flex h-[34px] items-center gap-2 border-b border-line bg-panel px-3">
        <Icon name="terminal-window" className="text-fg-dim" />
        <span className="flex-1 font-medium">{title}</span>
        {live ? (
          <span className="inline-flex items-center gap-1.5 text-[12px] text-amber">
            <i className="size-1.5 animate-pulse rounded-full bg-amber" />
            Live
          </span>
        ) : null}
        {actions ?? null}
      </div>
      <div
        role="log"
        className="overflow-auto px-3 py-2 font-mono text-[12px] leading-[19px]"
        style={height ? { height } : undefined}
      >
        {lines.map((l, i) => (
          <div key={i} className="grid grid-cols-[64px_1fr] gap-3 whitespace-pre-wrap text-fg-dim">
            <span className="text-fg-mute tabular-nums">{l.t}</span>
            <span className={l.level ? LEVEL[l.level] : undefined}>{l.text}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
