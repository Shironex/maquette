import { cloneElement, isValidElement, type ReactElement, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { cn } from '../../lib/cn';
import { Kbd } from './Kbd';
import { useTooltip } from './Tooltip.hooks';
import type { TooltipSide } from './tooltipPlacement';

export interface TooltipProps {
  label: ReactNode;
  /** A second, quieter line: why a control is disabled, say. Wraps. */
  detail?: ReactNode;
  /** Keycaps after the label: one key ("W") or a chord (["⌘", "Z"]). */
  shortcut?: string | readonly string[];
  side?: TooltipSide;
  /** Forces it shown (or hidden), for pictures and tests. */
  open?: boolean;
  /** Wraps a longer sentence at a fixed width instead of one line. */
  wrap?: boolean;
  /** Never shows; the trigger stays where it is in the tree. */
  disabled?: boolean;
  /** The hover delay in ms; inside the delay group it shows at once. */
  delay?: number;
  /** One element, the trigger. It keeps its own accessible name. */
  children: ReactNode;
  className?: string;
}

/** Where the bubble slides in from, per side. */
const SLIDE: Record<TooltipSide, string> = {
  top: '0px, 4px',
  bottom: '0px, -4px',
  left: '4px, 0px',
  right: '-4px, 0px',
};

type Described = ReactElement<{ 'aria-describedby'?: string }>;

/**
 * A hover and keyboard focus label in a bubble over the window, flipped and
 * pushed so it never clips at an edge. While it shows it describes the
 * trigger (aria-describedby). Escape, a press, leaving and blur hide it.
 */
export function Tooltip({
  label,
  detail,
  shortcut,
  side = 'top',
  open,
  wrap,
  disabled,
  delay,
  children,
  className,
}: TooltipProps) {
  const content = `${String(label)} ${String(detail ?? '')}`;
  const tip = useTooltip({ side, open, disabled, delay, content });
  const own = isValidElement(children) ? (children as Described).props['aria-describedby'] : null;
  const describedBy = [own, tip.id].filter(Boolean).join(' ');
  const trigger =
    tip.visible && isValidElement(children)
      ? cloneElement(children as Described, { 'aria-describedby': describedBy })
      : children;
  const slide = SLIDE[tip.place?.side ?? side];
  const bubble = tip.visible ? (
    <span
      ref={tip.bubble}
      id={tip.id}
      role="tooltip"
      data-side={tip.place?.side ?? side}
      style={{
        top: tip.place?.top ?? 0,
        left: tip.place?.left ?? 0,
        visibility: tip.place ? undefined : 'hidden',
        ['--tip-slide' as string]: slide,
      }}
      className={cn(
        'pointer-events-none fixed z-[90] flex animate-[tooltip-in_120ms_ease-out] items-center gap-2 rounded-control bg-fg px-2 py-[5px] text-[12px] leading-4 font-medium text-app shadow-raised',
        wrap || detail ? 'w-max max-w-[260px] whitespace-normal' : 'whitespace-nowrap',
        className
      )}
    >
      <span className="flex min-w-0 flex-col">
        <span>{label}</span>
        {detail ? <span className="font-normal text-app/75">{detail}</span> : null}
      </span>
      {shortcut ? <Kbd keys={shortcut} tone="inverse" /> : null}
    </span>
  ) : null;
  const portal =
    bubble && typeof document !== 'undefined' ? createPortal(bubble, document.body) : null;

  return (
    <span ref={tip.anchor} className="contents" {...(disabled ? null : tip.handlers)}>
      {trigger}
      {portal}
    </span>
  );
}
