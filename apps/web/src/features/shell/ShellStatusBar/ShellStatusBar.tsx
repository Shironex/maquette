import { StatusBar, type StatusDot } from '@maquette/ui';
import type { CodexUsage } from '@/ipc/client';
import { CodexUsageItem } from '../CodexUsageItem';
import { SiteServerItem } from '../SiteServerItem';
import { useRunAnnouncement, useShellStatusBar } from './ShellStatusBar.hooks';

const siteControl = (label: string, dot: StatusDot) => <SiteServerItem label={label} dot={dot} />;
const codexControl = (usage: CodexUsage) => <CodexUsageItem usage={usage} />;

export default function ShellStatusBar() {
  const items = useShellStatusBar(siteControl, codexControl);
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
