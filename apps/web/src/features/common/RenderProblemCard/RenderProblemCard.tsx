import { Button, Icon, cn, type IconName } from '@maquette/ui';
import type { RenderProblem, RenderStyle } from '@/ipc/client';
import { problemCard } from '@/state/renders';

export interface ProblemAction {
  label: string;
  icon: IconName;
  onClick: () => void;
}

export interface RenderProblemCardProps {
  problem: RenderProblem;
  name: string;
  style: RenderStyle;
  bytes?: number | null;
  actions?: readonly ProblemAction[];
}

const TONE = {
  danger: 'border-danger/28 bg-danger-soft [&>h4>svg]:text-danger',
  warning: 'border-amber/28 bg-amber-soft [&>h4>svg]:text-amber',
} as const;

/** One thing that went wrong with a render, and what to do next. */
export default function RenderProblemCard({
  problem,
  name,
  style,
  bytes,
  actions = [],
}: RenderProblemCardProps) {
  const { title, tone } = problemCard(problem, name, style, bytes);
  const buttons = actions.map((action, i) => (
    <Button
      key={action.label}
      size="sm"
      variant={i === 0 ? 'secondary' : 'ghost'}
      icon={action.icon}
      onClick={action.onClick}
    >
      {action.label}
    </Button>
  ));
  return (
    <article className={cn('flex flex-col gap-1.5 rounded-card border p-3', TONE[tone])}>
      <h4 className="flex items-center gap-2 font-semibold">
        <Icon name={tone === 'danger' ? 'warning-circle' : 'warning'} size={15} />
        {title}
      </h4>
      <p className="text-meta text-fg-dim">{problem.message}</p>
      {actions.length > 0 ? <div className="mt-1 flex flex-wrap gap-1.5">{buttons}</div> : null}
    </article>
  );
}
