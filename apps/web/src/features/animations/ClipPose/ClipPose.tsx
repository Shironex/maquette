import { cn } from '@maquette/ui';
import type { RestBone } from '@/ipc/client';
import { restPoseLines } from '@/state/animations';

export interface ClipPoseProps {
  bones: readonly RestBone[];
  className?: string;
}

const SIZE = 100;

/** A clip's skeleton in its rest pose, from the front: no model is needed to see a clip. */
export default function ClipPose({ bones, className }: ClipPoseProps) {
  const lines = restPoseLines(bones, SIZE);
  const segments = lines.map((l, i) => (
    <line key={i} {...l} className="stroke-fg-dim" strokeWidth={1.6} strokeLinecap="round" />
  ));
  const joints = lines.map((l, i) => (
    <circle key={i} cx={l.x1} cy={l.y1} r={1.6} className="fill-clay-text" />
  ));
  return (
    <span data-bones={bones.length} className={cn('grid place-items-center bg-studio', className)}>
      <svg viewBox={`0 0 ${SIZE} ${SIZE}`} aria-hidden="true" className="h-[86%] w-[86%]">
        <line x1={8} x2={92} y1={94} y2={94} className="stroke-line-strong" strokeWidth={0.8} />
        {segments}
        {joints}
      </svg>
    </span>
  );
}
