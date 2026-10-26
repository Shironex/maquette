import { FrameStatus } from '../FrameStatus';
import { PageFrame } from '../PageFrame';
import { usePagePreview } from './PagePreview.hooks';

/**
 * The project's page in a small frame, for a view that drives its scroll:
 * the Timeline seeks it through the page bridge as its playhead moves.
 */
export default function PagePreview() {
  const p = usePagePreview();
  return (
    <PageFrame
      id="main"
      src={p.source.src}
      address={p.source.address}
      badge={p.badge}
      device={p.device}
      label={p.label}
      compact
      className="size-full"
    >
      {p.status ? <FrameStatus state={p.status} site={p.site} command={p.command} /> : null}
    </PageFrame>
  );
}
