import { StatusBar, type StatusDot } from '@maquette/ui';
import { SiteServerItem } from '../SiteServerItem';
import { useRunAnnouncement, useShellStatusBar } from './ShellStatusBar.hooks';

const siteControl = (label: string, dot: StatusDot) => <SiteServerItem label={label} dot={dot} />;

export default function ShellStatusBar() {
  const items = useShellStatusBar(siteControl);
  const run = useRunAnnouncement();
  return (
    <>
      <StatusBar items={items} />
      {/* The run's step, read out as it moves; the bar is not live, its times tick. */}
      <p role="status" className="sr-only">
        {run}
      </p>
    </>
  );
}
