import { Icon, IconButton, cn, type IconName } from '@maquette/ui';
import { PageHotspots } from '../PageHotspots';
import { usePageFrame } from './PageFrame.hooks';
import type { FrameBadge, PageFrameProps } from './PageFrame.types';

const BADGES: Record<FrameBadge, { label: string; icon?: IconName }> = {
  live: { label: 'Live' },
  poster: { label: 'Poster', icon: 'image' },
  concept: { label: 'Mock', icon: 'file-text' },
  run: { label: 'Still', icon: 'image' },
};

/**
 * One page in a browser frame: the address bar, then the page laid out at
 * the device's width and scaled to fit. The page runs in a sandbox that
 * keeps it from navigating the app or opening windows.
 */
export default function PageFrame(props: PageFrameProps) {
  const { src, address, badge, label, showStop, image, actions, children, className } = props;
  const compact = props.layout === 'tile' || props.layout === 'column';
  const { frame, viewport, d, scale, fitText, reloadKey, reload, pin, hotspots } =
    usePageFrame(props);
  const shown = BADGES[badge];
  const showHotspots = props.id === 'main' && hotspots && badge === 'live';
  const width = d.width * scale;
  const height = d.height * scale;
  const stopPin = showStop && src ? pin : null;
  const hasPage = Boolean(src || image);
  const showFit = !compact && hasPage;
  const byWidth = props.layout === 'column';
  return (
    <figure
      aria-label={label}
      className={cn(
        'relative m-0 flex min-h-0 min-w-0 flex-col overflow-hidden rounded-[12px] border border-line-strong bg-[#0f1011] shadow-[0_20px_50px_-24px_rgba(10,9,8,0.9)]',
        className
      )}
    >
      <div
        className={cn(
          'flex h-8 flex-none items-center border-b border-line bg-raised font-mono text-[11.5px] text-fg-dim',
          compact ? 'gap-1.5 px-2' : 'gap-2 px-2.5'
        )}
      >
        <span aria-hidden="true" className="flex flex-none items-center gap-1.5">
          <i className="size-2.5 rounded-full bg-[#ff5f57]" />
          <i className="size-2.5 rounded-full bg-[#febc2e]" />
          <i className="size-2.5 rounded-full bg-[#28c840]" />
        </span>
        <span className="flex h-[22px] min-w-0 flex-1 items-center gap-1.5 overflow-hidden rounded-control bg-app px-2 whitespace-nowrap">
          <Icon name="lock-simple" size={12} className="flex-none text-fg-mute" />
          <span className="truncate">{address}</span>
        </span>
        {hasPage ? (
          <span className="inline-flex h-[22px] flex-none items-center gap-[5px] rounded-control bg-[rgba(28,27,25,0.78)] px-2 font-sans text-[11.5px] font-medium text-fg">
            {shown.icon ? (
              <Icon name={shown.icon} size={12} className="text-fg-dim" />
            ) : (
              <span aria-hidden="true" className="size-1.5 rounded-full bg-sage" />
            )}
            {shown.label}
          </span>
        ) : null}
        {actions}
        <IconButton
          icon="arrow-clockwise"
          label="Reload the page"
          size="sm"
          disabled={!src}
          onClick={reload}
        />
      </div>
      <div
        ref={viewport}
        className={cn(
          'relative flex min-h-0 justify-center overflow-hidden',
          byWidth ? 'flex-initial items-start' : 'flex-1 items-center'
        )}
        style={byWidth ? { aspectRatio: `${d.width} / ${d.height}` } : undefined}
      >
        {image ? (
          <img src={image} alt="" className="size-full object-cover" />
        ) : src ? (
          <div className="relative flex-none overflow-hidden" style={{ width, height }}>
            <iframe
              key={reloadKey}
              ref={frame}
              src={src}
              title={label}
              sandbox="allow-scripts allow-same-origin allow-forms allow-pointer-lock"
              referrerPolicy="no-referrer"
              className="absolute top-0 left-0 origin-top-left border-0 bg-white"
              style={{ width: d.width, height: d.height, transform: `scale(${scale})` }}
            />
            {showHotspots ? <PageHotspots /> : null}
          </div>
        ) : (
          children
        )}
        {stopPin ? (
          <span className="absolute bottom-3 left-3 inline-flex h-6 items-center gap-1.5 rounded-pill border border-focus bg-[rgba(28,27,25,0.9)] pr-[9px] pl-1 text-[11.5px] font-medium whitespace-nowrap text-fg">
            <b className="grid size-4 place-items-center rounded-full bg-focus font-mono text-[10px] text-app">
              {stopPin.number}
            </b>
            {stopPin.name}
          </span>
        ) : null}
        {showFit ? (
          <span className="absolute right-3 bottom-3 inline-flex h-[22px] items-center gap-[5px] rounded-control bg-[rgba(28,27,25,0.78)] px-2 text-[11.5px] font-medium text-fg">
            <Icon name={d.icon} size={12} className="text-fg-dim" />
            {fitText}
          </span>
        ) : null}
      </div>
    </figure>
  );
}
