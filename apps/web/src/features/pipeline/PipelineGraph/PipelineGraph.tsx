import { Fragment } from 'react';
import { IconButton, PipelineNode, cn, type IconName } from '@maquette/ui';
import type { PipelineView, StepKind } from '@/ipc/client';
import { nodeViews, stepCaption } from '@/state/pipeline';

const KIND_ICON: Record<StepKind, IconName> = {
  blender: 'cube-transparent',
  shell: 'terminal-window',
  'gltf-transform': 'package',
  toktx: 'image',
  ffmpeg: 'film-strip',
};

export interface PipelineGraphProps {
  view: PipelineView;
  busy: boolean;
  onRunFrom: (step: string) => void;
}

/** The steps left to right, in definition order, with edges between them. */
export default function PipelineGraph({ view, busy, onRunFrom }: PipelineGraphProps) {
  const nodes = nodeViews(view);
  const steps = view.def.steps.map((step, i) => {
    const node = nodes[step.id] ?? { state: 'waiting' as const };
    const prev = i > 0 ? nodes[view.def.steps[i - 1]?.id ?? ''] : undefined;
    const edge =
      node.state === 'running'
        ? 'bg-amber shadow-[0_0_0_2px_var(--amber-soft)]'
        : prev && (prev.state === 'done' || prev.state === 'cached')
          ? 'bg-sage/60'
          : 'bg-line-strong';
    return (
      <Fragment key={step.id}>
        {i > 0 ? (
          <div aria-hidden="true" className="flex w-6 flex-none items-center">
            <span className={cn('h-0.5 w-full rounded-full', edge)} />
          </div>
        ) : null}
        <PipelineNode
          step={i + 1}
          name={step.name}
          icon={KIND_ICON[step.kind]}
          script={stepCaption(step)}
          state={node.state}
          progress={node.progress}
          label={node.label}
          duration={node.duration}
          detail={node.detail}
          className="min-w-[156px] flex-1"
          actions={
            <IconButton
              icon="play"
              size="sm"
              variant="ghost"
              label={`Run from ${step.name}`}
              title="Run from here"
              disabled={busy}
              onClick={() => onRunFrom(step.id)}
            />
          }
        />
      </Fragment>
    );
  });
  return (
    <div
      aria-label="Pipeline steps"
      className="flex items-stretch overflow-x-auto rounded-card border border-line bg-panel px-3.5 py-[18px]"
    >
      {steps}
    </div>
  );
}
