import { Fragment } from 'react';
import { Button, Icon, cn } from '@maquette/ui';
import { SectionLabel } from '@/features/common/SectionLabel';
import { useStatesStrip } from './StatesStrip.hooks';

/** The model's states in order, the move into each between them, and Add state. */
export default function StatesStrip() {
  const strip = useStatesStrip();
  if (!strip) return null;
  const cards = strip.cards.map((card, i) => (
    <Fragment key={card.id}>
      {i > 0 ? (
        <span
          className={cn(
            'flex w-[68px] flex-none flex-col items-center gap-0.5 rounded-control border border-line bg-raised px-1 py-1.5 font-mono text-[11px] leading-[13px] whitespace-nowrap',
            !card.into && 'text-fg-mute'
          )}
        >
          <Icon name="arrow-right" size={12} className="text-fg-dim" />
          {card.into ? (
            <>
              <span>{card.into.time}</span>
              <span className="text-fg-dim">{card.into.word}</span>
            </>
          ) : (
            <span>cut</span>
          )}
        </span>
      ) : null}
      <button
        type="button"
        aria-pressed={card.current}
        onClick={() => strip.selectState(card.id)}
        className={cn(
          'flex w-[128px] flex-none cursor-pointer flex-col gap-1.5 rounded-card border border-line bg-raised p-2 text-left hover:bg-hover',
          card.current && 'border-clay-line bg-clay-soft hover:bg-clay-soft'
        )}
      >
        <span className="grid h-[52px] place-items-center rounded-[6px] bg-studio">
          <Icon name="selection" size={18} className="text-fg-mute" />
        </span>
        <span className="truncate text-[13px] leading-[18px] font-semibold">{card.name}</span>
        <span className="font-mono text-[11.5px] text-fg-mute">{card.line}</span>
      </button>
    </Fragment>
  ));
  return (
    <section
      aria-label="States"
      className="flex flex-none flex-col gap-3 rounded-card border border-line bg-panel p-3"
    >
      <div className="flex items-center gap-3">
        <SectionLabel className="whitespace-nowrap">States · {strip.cards.length}</SectionLabel>
        <span className="flex-1" />
        <span className="truncate text-meta text-fg-mute">
          Each state is a pose of the parts. Pick one to time the move into it.
        </span>
        <Button
          size="sm"
          icon="plus"
          onClick={strip.add}
          disabled={strip.full}
          disabledReason="Sixteen states at most"
        >
          Add state
        </Button>
      </div>
      <div className="flex items-center gap-2 overflow-x-auto pb-1">{cards}</div>
    </section>
  );
}
