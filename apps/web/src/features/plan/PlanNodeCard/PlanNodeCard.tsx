import { Handle, Position, type NodeProps } from '@xyflow/react';
import { cn, Icon, IconButton, type IconName } from '@maquette/ui';
import type { NodeKind } from '@/ipc/client';
import { hostOf, isLinked, LIMITS, type FlowNode } from '@/state/plans';
import { LinkedBody } from './LinkedBody';
import { NodeExtras, OverlapFlags } from './NodeExtras';
import { StopBody } from './StopBody';
import { usePlanNodeCard } from './PlanNodeCard.hooks';

type Plain = Exclude<NodeKind, 'asset' | 'direction' | 'stop'>;

const KIND: Record<Plain, { label: string; icon: IconName }> = {
  note: { label: 'Note', icon: 'note-pencil' },
  image: { label: 'Image', icon: 'image' },
  link: { label: 'Link', icon: 'link-simple' },
};

const HANDLES = [
  { id: 'top', position: Position.Top },
  { id: 'right', position: Position.Right },
  { id: 'bottom', position: Position.Bottom },
  { id: 'left', position: Position.Left },
] as const;

/**
 * One node on a canvas, the same parts for every kind: the kind line, a
 * title, the body (text, picture, link, or an asset or Lab direction read
 * live) and a quiet footer, with a handle on each side to link it to
 * another node. A double click edits a note's, picture's or link's text.
 */
export default function PlanNodeCard({ data, selected }: NodeProps<FlowNode>) {
  const { node } = data;
  const v = usePlanNodeCard(node);
  const field = (what: 'text' | 'title', label: string, rows: number) => (
    <textarea
      aria-label={label}
      autoFocus
      rows={rows}
      maxLength={what === 'text' ? LIMITS.text : LIMITS.title}
      value={v.value}
      onChange={e => v.setValue(e.target.value)}
      onBlur={v.commit}
      onKeyDown={e => {
        e.stopPropagation();
        if (e.key === 'Escape') v.cancel();
        if (e.key !== 'Enter' || !(e.metaKey || e.ctrlKey || what === 'title')) return;
        e.preventDefault();
        v.commit();
      }}
      className="nodrag nowheel mx-2 mt-1 resize-none rounded-[6px] border border-line-strong bg-app px-2 py-1.5 text-[12.5px] leading-[17px] text-fg outline-none focus:border-clay"
    />
  );
  const linked = isLinked(node.kind);
  const marked = v.mark ? (
    <span className="plan-badge">
      <Icon name={v.mark === 'added' ? 'sparkle' : 'note-pencil'} size={12} />
      {v.mark}
      <span className="sr-only"> by {node.by ?? 'an outside change'}</span>
    </span>
  ) : null;
  const badge = <NodeExtras node={node} selected={Boolean(selected)} fallback={marked} />;
  const agent = node.by !== null && 'is-agent';
  const handles = HANDLES.map(h => (
    <Handle
      key={h.id}
      id={h.id}
      type="source"
      position={h.position}
      className={cn('plan-handle', linked && 'plan-handle-diamond')}
    />
  ));
  if (node.kind === 'asset' || node.kind === 'direction' || node.kind === 'stop') {
    return (
      <div
        className={cn('plan-node', v.gone && 'plan-node-gone', agent, selected && 'is-selected')}
      >
        {handles}
        {badge}
        {node.kind === 'stop' ? <StopBody node={node} /> : <LinkedBody node={node} />}
      </div>
    );
  }
  const kind = KIND[node.kind];
  return (
    <div
      className={cn(
        'plan-node',
        node.kind === 'note' && 'plan-node-note',
        agent,
        selected && 'is-selected'
      )}
      onDoubleClick={e => {
        e.stopPropagation();
        v.start(node.kind === 'note' ? 'text' : 'title');
      }}
    >
      {handles}
      {badge}
      <div className="flex items-center gap-1.5 px-2.5 pt-2 text-[10.5px] leading-[14px] font-medium tracking-[0.05em] text-fg-mute uppercase">
        <Icon name={kind.icon} size={12} />
        {kind.label}
        {node.kind === 'link' ? (
          <span className="ml-auto truncate font-mono text-[11px] tracking-normal normal-case">
            {hostOf(node.url)}
          </span>
        ) : node.by ? (
          <span className="ml-auto truncate tracking-normal text-[var(--plan-agent)] normal-case">
            {node.by}
          </span>
        ) : null}
      </div>
      {node.kind === 'image' ? (
        <div className="mx-2 mt-2 aspect-[4/3] overflow-hidden rounded-[7px] bg-studio">
          {v.src ? (
            <img
              src={v.src}
              alt={node.title || 'Picture'}
              className="block size-full object-cover"
              draggable={false}
            />
          ) : (
            <div className="grid size-full place-items-center text-meta text-fg-mute">
              Picture not found
            </div>
          )}
        </div>
      ) : null}
      {v.editing === 'title' ? (
        field('title', node.kind === 'image' ? 'Caption' : 'Title', 2)
      ) : node.title ? (
        <p className="px-2.5 pt-1 text-[13px] leading-[17px] font-semibold text-fg">{node.title}</p>
      ) : null}
      {v.editing === 'text' ? (
        field('text', 'Note text', 5)
      ) : node.text ? (
        <p className="px-2.5 pt-1 pb-2.5 text-[12px] leading-4 whitespace-pre-wrap text-fg-dim">
          {node.text}
        </p>
      ) : node.kind === 'note' && !node.title ? (
        <p className="px-2.5 pt-1 pb-2.5 text-[12px] leading-4 text-fg-mute">
          Double click to write
        </p>
      ) : (
        <div className="h-2.5" />
      )}
      {node.kind === 'note' ? <OverlapFlags id={node.id} /> : null}
      {node.kind === 'link' ? (
        <div className="flex items-center gap-1 border-t border-line px-2 py-1">
          <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-fg-mute">
            {node.url}
          </span>
          <IconButton
            icon="copy"
            label="Copy the address"
            size="sm"
            variant="ghost"
            className="nodrag"
            onClick={v.copyLink}
          />
          <IconButton
            icon="arrow-square-out"
            label="Open in the browser"
            size="sm"
            variant="ghost"
            className="nodrag"
            disabled={!v.canOpen}
            disabledReason="Only https pages open from a canvas"
            onClick={v.openLink}
          />
        </div>
      ) : null}
    </div>
  );
}
