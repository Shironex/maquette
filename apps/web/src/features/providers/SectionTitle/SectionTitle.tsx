import type { ReactNode } from 'react';

export interface SectionTitleProps {
  children: string;
  aside?: ReactNode;
}

/** A small uppercase heading for a section of a Send to 3D sheet. */
export default function SectionTitle({ children, aside }: SectionTitleProps) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <h3 className="text-[11px] leading-[14px] font-medium tracking-[0.04em] text-fg-mute uppercase">
        {children}
      </h3>
      {aside ? <span className="text-meta text-fg-mute">{aside}</span> : null}
    </div>
  );
}
