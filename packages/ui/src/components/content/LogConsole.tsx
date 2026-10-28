import type { ReactNode } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useLogLines } from './LogConsole.hooks';

export interface LogLine {
  t: string;
  text: string;
  level?: 'info' | 'ok' | 'warn' | 'error';
  /** Fades the line, for output nobody needs to read. */
  dim?: boolean;
  /** Replaces `text` when part of the line is styled on its own. */
  node?: ReactNode;
}

export interface LogConsoleProps {
  title?: string;
  live?: boolean;
  height?: number;
  actions?: ReactNode;
  lines: readonly LogLine[];
  /** Keeps the newest line in view while lines arrive. */
  autoScroll?: boolean;
  /** Shown when there are no lines. */
  empty?: ReactNode;
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
  autoScroll,
  empty,
  className,
}: LogConsoleProps) {
  const { body, list, slice } = useLogLines(autoScroll, lines);
  const lineRows = lines.slice(slice.start, slice.end).map((l, n) => (
    <div
      key={slice.start + n}
      {...slice.itemProps(slice.start + n)}
      className="grid grid-cols-[64px_1fr] gap-3 whitespace-pre-wrap text-fg-dim"
    >
      <span className="text-fg-mute tabular-nums">{l.t}</span>
      <span className={l.dim ? 'text-fg-mute' : l.level ? LEVEL[l.level] : undefined}>
        {l.node ?? l.text}
      </span>
    </div>
  ));
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
        ref={body}
        role="log"
        aria-label={title}
        // Focusable, so the keyboard can scroll it.
        tabIndex={0}
        className="min-h-0 flex-1 overflow-auto px-3 py-2 font-mono text-[12px] leading-[19px]"
        style={height ? { height } : undefined}
      >
        {lines.length === 0 && empty ? <div className="text-fg-mute">{empty}</div> : null}
        <div ref={list} style={{ paddingTop: slice.before, paddingBottom: slice.after }}>
          {lineRows}
        </div>
      </div>
    </div>
  );
}
