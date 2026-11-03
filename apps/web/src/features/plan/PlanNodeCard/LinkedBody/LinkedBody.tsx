import { Button, cn, Icon, type IconName } from '@maquette/ui';
import type { PlanNode } from '@/ipc/client';
import type { LinkTone } from '@/state/plan-links';
import { useLinkedBody } from './LinkedBody.hooks';

const TONE: Record<LinkTone, { text: string; icon: IconName }> = {
  none: { text: 'text-fg-mute', icon: 'circle' },
  run: { text: 'text-amber', icon: 'circle-notch' },
  pick: { text: 'text-clay-text', icon: 'star' },
  look: { text: 'text-amber', icon: 'warning' },
  ok: { text: 'text-sage', icon: 'check-circle' },
  bad: { text: 'text-danger', icon: 'warning-circle' },
};

/**
 * The body of an asset or Lab direction node, all read live from the
 * project: the kind line with one fact, the picture, the name, the state
 * with its colour and icon, one quiet line, and the state's next step.
 * The file keeps only the id, so a removed asset says so here.
 */
export default function LinkedBody({ node }: { node: PlanNode }) {
  const { view, src, go } = useLinkedBody(node);
  const tone = TONE[view.tone];
  const isAsset = node.kind === 'asset';
  const picture = src ? (
    <img src={src} alt="" className="block size-full object-cover" draggable={false} />
  ) : (
    <Icon name={view.gone ? 'question' : isAsset ? 'cube' : 'flask'} size={isAsset ? 18 : 22} />
  );
  const frame = cn(
    'shrink-0 overflow-hidden',
    !src && 'grid place-items-center border border-dashed border-line-strong text-fg-mute'
  );
  return (
    <>
      <div className="flex items-center gap-1.5 px-2.5 pt-2 text-[10.5px] leading-[14px] font-medium tracking-[0.05em] text-fg-mute uppercase">
        <Icon name={isAsset ? 'cube' : 'flask'} size={12} />
        {isAsset ? 'Asset' : 'Lab direction'}
        {view.fact ? (
          <span className="ml-auto truncate font-mono text-[11px] tracking-normal normal-case">
            {view.fact}
          </span>
        ) : null}
      </div>
      {isAsset ? null : (
        <div className={cn(frame, 'mx-2 mt-2 h-[84px] rounded-[7px] bg-studio')}>{picture}</div>
      )}
      <div className="flex items-center gap-2.5 px-2.5 pt-2">
        {isAsset ? <span className={cn(frame, 'size-10 rounded-[7px]')}>{picture}</span> : null}
        <div className="min-w-0">
          <p
            className={cn(
              'truncate text-[13px] leading-[17px] font-semibold',
              view.gone ? 'text-fg-dim' : 'text-fg'
            )}
          >
            {view.name}
          </p>
          <p
            className={cn('flex items-center gap-1 text-[11.5px] leading-4 font-medium', tone.text)}
          >
            <Icon name={tone.icon} size={13} />
            <span className="truncate">{view.status}</span>
          </p>
        </div>
      </div>
      {view.detail ? (
        <p className="truncate px-2.5 pt-1.5 font-mono text-[11px] leading-[14px] text-fg-mute">
          {view.detail}
        </p>
      ) : null}
      {view.help ? (
        <p className="px-2.5 pt-1 text-[11.5px] leading-4 text-fg-dim">{view.help}</p>
      ) : null}
      <div className="px-2 pt-2 pb-2">
        <Button variant="ghost" size="sm" className="nodrag" onClick={go}>
          {view.action.label}
        </Button>
      </div>
    </>
  );
}
