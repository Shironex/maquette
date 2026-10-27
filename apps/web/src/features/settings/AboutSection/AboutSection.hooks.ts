import { useCallback, useEffect, useState } from 'react';
import { commands, errorMessage, type AboutInfo } from '@/ipc/client';
import { blenderVersion, reportUrl, versionBlock } from '@/state/about';
import { useAppStore } from '@/state/use-app-store';
import { useUpdaterStore } from '@/state/use-updater-store';
import { fromStatus } from '@/state/updater';
import { copyText } from '@/features/config/copyText';

/** How long "Copied" stays on the button. */
const COPIED_MS = 2000;

/** The build and machine facts, the copy block and the links About opens. */
export function useAboutSection() {
  const [about, setAbout] = useState<AboutInfo | null>(null);
  const [copied, setCopied] = useState(false);
  const checked = useAppStore(s => s.tools.reports.find(r => r.id === 'blender')?.version ?? null);
  const notify = useAppStore(s => s.notify);

  useEffect(() => {
    let live = true;
    commands.appAbout().then(
      info => {
        if (!live) return;
        setAbout(info);
        // Settings opened before the shell read the updater: start from here.
        if (useUpdaterStore.getState().mode === null)
          useUpdaterStore.setState(fromStatus(info.updater));
      },
      () => undefined
    );
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), COPIED_MS);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const blender = about ? blenderVersion(about, checked) : null;

  const open = useCallback(
    (url: string) => {
      commands.appOpenLink(url).catch((error: unknown) =>
        notify({
          status: 'failed',
          title: 'Could not open the link',
          detail: errorMessage(error),
        })
      );
    },
    [notify]
  );

  const copy = () => {
    if (!about) return;
    void copyText(versionBlock(about, blender)).then(ok => {
      if (ok) setCopied(true);
      else notify({ status: 'failed', title: 'Could not copy the version info' });
    });
  };

  const report = () => {
    if (about) open(reportUrl(about, blender));
  };

  return { about, blender, copied, copy, report, open };
}
