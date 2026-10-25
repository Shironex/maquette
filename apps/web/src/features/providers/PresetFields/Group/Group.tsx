import type { ReactNode } from 'react';

export interface GroupProps {
  title: string;
  children: ReactNode;
}

/** A titled box for the rig and animation fields. */
export default function Group({ title, children }: GroupProps) {
  return (
    <div className="flex flex-col gap-2.5 rounded-[8px] border border-line bg-raised px-3 py-2.5">
      <div className="font-semibold">{title}</div>
      {children}
    </div>
  );
}
