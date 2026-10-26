import { Button, Icon } from '@maquette/ui';
import type { ReactNode } from 'react';
import { SectionLabel } from '@/features/common/SectionLabel';
import { useOverBudget } from './OverBudget.hooks';

function Way({ title, detail, action }: { title: string; detail: string; action: ReactNode }) {
  return (
    <div className="flex items-center gap-3 border-t border-line py-2.5">
      <div className="min-w-0 flex-1">
        <p className="text-[13px] leading-[18px] font-semibold">{title}</p>
        <p className="text-meta text-fg-mute">{detail}</p>
      </div>
      {action}
    </div>
  );
}

/** The scene over its budget with the clips in it, and what fits it again. */
export default function OverBudget() {
  const over = useOverBudget();
  if (!over) return null;
  const ways = over.ways.map(way => (
    <Way
      key={way.id}
      title={way.title}
      detail={way.detail}
      action={
        <Button
          size="sm"
          disabled={!over.supported}
          disabledReason="Update the starter scripts first"
          onClick={way.apply}
        >
          Leave it off
        </Button>
      }
    />
  ));
  return (
    <section aria-label="Over budget" className="flex flex-col gap-3">
      <div
        role="alert"
        className="flex gap-2.5 rounded-card border border-danger/40 bg-danger-soft p-3.5"
      >
        <Icon name="x-circle" size={16} className="mt-0.5 flex-none text-danger" />
        <div>
          <p className="text-[13px] leading-[18px] font-semibold">{over.title}</p>
          <p className="mt-0.5 text-meta text-fg-dim">{over.detail}</p>
        </div>
      </div>
      <div className="flex items-center justify-between">
        <SectionLabel>Ways to fit it</SectionLabel>
        <span className="text-meta text-fg-mute">pick one or more</span>
      </div>
      <div className="flex flex-col">
        {ways}
        <Way
          title="Pack the keys with meshopt"
          detail="The Compress step packs animation keys too. Turn it up per model in the Pipeline view."
          action={
            <Button size="sm" onClick={over.openPipeline}>
              Open Pipeline
            </Button>
          }
        />
      </div>
    </section>
  );
}
