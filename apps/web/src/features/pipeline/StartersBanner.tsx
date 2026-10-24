import { useState } from 'react';
import { Button, InlineBanner } from '@maquette/ui';
import type { PipelineView } from '@/ipc/client';
import { readFlag, writeFlag } from '@/lib/storage';
import { fileName, listNames, starterOffer } from '@/state/pipeline';
import { usePipelineStore } from '@/state/pipeline-store';

/** "Keep mine" for an edited script, remembered per project and script. */
const keepKey = (project: string, path: string) => `maquette.starter.keep.${project}.${path}`;

/**
 * Starter scripts that this build ships newer versions of. Untouched old
 * copies update in one click; an edited script is never replaced unless
 * asked, one at a time, with the old file kept as `<name>.bak`.
 */
export function StartersBanner({ view, busy }: { view: PipelineView; busy: boolean }) {
  const updateStarters = usePipelineStore(s => s.updateStarters);
  const [working, setWorking] = useState<string | null>(null);
  const [kept, setKept] = useState<readonly string[]>([]);
  const { update, edited } = starterOffer(view.starters);
  const shown = edited.filter(p => !kept.includes(p) && !readFlag(keepKey(view.project, p)));
  if (update.length === 0 && shown.length === 0) return null;

  const run = async (id: string, scripts: string[], replace: boolean) => {
    setWorking(id);
    await updateStarters(scripts, replace);
    setWorking(null);
  };
  const keep = (path: string) => {
    writeFlag(keepKey(view.project, path), true);
    setKept(k => [...k, path]);
  };
  const blocked = busy ? 'Wait for the run to finish' : undefined;

  return (
    <div className="flex flex-col gap-2">
      {update.length > 0 ? (
        <InlineBanner
          tone="info"
          title="New starter scripts available"
          detail={`${listNames(update)} ${update.length === 1 ? 'is an untouched copy' : 'are untouched copies'} of an older starter. Updating brings in what newer starters do, for example a real light bake.`}
          actions={
            <Button
              size="sm"
              variant="primary"
              loading={working === 'all'}
              disabled={busy || working !== null}
              title={blocked}
              onClick={() => void run('all', update, false)}
            >
              Update
            </Button>
          }
        />
      ) : null}
      {shown.map(path => (
        <InlineBanner
          key={path}
          tone="info"
          title={`${fileName(path)} was edited`}
          detail={`A newer starter exists. Your version stays as it is unless you replace it; the old file is kept as ${fileName(path)}.bak.`}
          actions={
            <>
              <Button size="sm" variant="ghost" onClick={() => keep(path)}>
                Keep mine
              </Button>
              <Button
                size="sm"
                loading={working === path}
                disabled={busy || working !== null}
                title={blocked}
                onClick={() => void run(path, [path], true)}
              >
                Replace with the new starter
              </Button>
            </>
          }
        />
      ))}
    </div>
  );
}
