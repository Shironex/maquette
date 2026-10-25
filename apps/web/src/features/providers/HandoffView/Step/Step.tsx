import type { ReactNode } from 'react';

export interface StepProps {
  n: number;
  title: string;
  children?: ReactNode;
}

export default function Step({ n, title, children }: StepProps) {
  return (
    <li className="flex gap-3">
      <span className="grid size-6 flex-none place-items-center rounded-full border-[1.5px] border-clay-line font-mono text-[11.5px] leading-none font-semibold text-clay-text">
        {n}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-2 pt-0.5">
        <div className="font-semibold">{title}</div>
        {children}
      </div>
    </li>
  );
}
