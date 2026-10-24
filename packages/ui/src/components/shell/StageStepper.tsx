import { Fragment } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';

export type StageState = 'done' | 'current' | 'todo' | 'running' | 'failed';

export interface Stage {
  id: string;
  label: string;
  state?: StageState;
}

export const PIPELINE_STAGES = ['Concept', 'Refs', 'Tripo', 'Blender', 'Web', 'Ship'] as const;

export interface StageStepperProps {
  stages?: readonly Stage[];
  onSelect?: (id: string) => void;
  className?: string;
}

const WORD: Record<StageState, string> = {
  done: 'done',
  current: 'current',
  todo: 'not started',
  running: 'running',
  failed: 'failed',
};

function Mark({ state }: { state: StageState }) {
  if (state === 'done') return <Icon name="check" size={14} className="text-sage" />;
  if (state === 'running')
    return <Icon name="circle-notch" size={14} spin className="text-amber" />;
  if (state === 'failed') return <Icon name="x-circle" size={14} className="text-danger" />;
  return (
    <span
      className={cn(
        'size-3.5 flex-none rounded-full border-[1.5px] border-line-strong',
        state === 'current' &&
          'border-clay-text bg-[radial-gradient(circle,var(--clay)_0_3px,transparent_3.5px)]'
      )}
    />
  );
}

export function StageStepper({ stages, onSelect, className }: StageStepperProps) {
  const list: readonly Stage[] =
    stages ?? PIPELINE_STAGES.map(l => ({ id: l.toLowerCase(), label: l }));
  return (
    <ol
      aria-label="Pipeline stages"
      className={cn('m-0 flex list-none items-center gap-0.5 p-0', className)}
    >
      {list.map((st, i) => {
        const state = st.state ?? 'todo';
        const pill = cn(
          'flex h-7 items-center gap-1.5 rounded-pill px-2.5 font-medium whitespace-nowrap text-fg-mute',
          state === 'done' && 'text-fg-dim',
          state === 'running' && 'text-amber',
          state === 'failed' && 'text-danger',
          state === 'current' && 'bg-clay-soft text-fg shadow-[inset_0_0_0_1px_var(--clay-line)]',
          onSelect && 'cursor-pointer hover:text-fg'
        );
        const body = (
          <>
            <Mark state={state} />
            {st.label}
            <span className="sr-only">{`, ${WORD[state]}`}</span>
          </>
        );
        return (
          <Fragment key={st.id}>
            {i > 0 ? (
              <li aria-hidden="true" className="flex text-line-strong">
                <Icon name="caret-right" size={12} />
              </li>
            ) : null}
            <li aria-current={state === 'current' ? 'step' : undefined} className="flex">
              {onSelect ? (
                <button type="button" className={pill} onClick={() => onSelect(st.id)}>
                  {body}
                </button>
              ) : (
                <span className={pill}>{body}</span>
              )}
            </li>
          </Fragment>
        );
      })}
    </ol>
  );
}
