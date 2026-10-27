import { Button, InlineBanner } from '@maquette/ui';
import type { PipelineView } from '@/ipc/client';
import { fileName, listNames } from '@/state/pipeline';
import { useStartersBanner } from './StartersBanner.hooks';

export interface StartersBannerProps {
  view: PipelineView;
  busy: boolean;
}

/**
 * Starter scripts that this build ships newer versions of. Untouched old
 * copies update in one click; an edited script is never replaced unless
 * asked, one at a time, with the old file kept as `<name>.bak`.
 */
export default function StartersBanner({ view, busy }: StartersBannerProps) {
  const { update, shown, working, run, keep, projectName } = useStartersBanner(view);
  if (update.length === 0 && shown.length === 0) return null;

  const blocked = busy ? 'Wait for the run to finish' : undefined;
  const editedBanners = shown.map(path => (
    <InlineBanner
      key={path}
      tone="info"
      title={`${fileName(path)} in ${projectName} was edited`}
      detail={`A newer starter exists. Your version stays as it is unless you replace it; the old file is kept as ${fileName(path)}.bak.`}
      actions={
        <>
          <Button
            size="sm"
            variant="ghost"
            tooltip="Leaves your edited file as it is and hides this notice"
            onClick={() => keep(path)}
          >
            Keep mine
          </Button>
          <Button
            size="sm"
            loading={working === path}
            disabled={busy || working !== null}
            disabledReason={blocked}
            tooltip="Overwrites your edited file with the new starter; your version is kept as a .bak"
            onClick={() => void run(path, [path], true)}
          >
            Replace with the new starter
          </Button>
        </>
      }
    />
  ));

  return (
    <div className="flex flex-col gap-2">
      {update.length > 0 ? (
        <InlineBanner
          tone="info"
          title={`New starter scripts for ${projectName}`}
          detail={`${listNames(update)} in ${projectName}'s tools folder ${update.length === 1 ? 'is an untouched copy' : 'are untouched copies'} of an older starter. Updating brings in what newer starters do, for example a real light bake.`}
          actions={
            <Button
              size="sm"
              variant="primary"
              loading={working === 'all'}
              disabled={busy || working !== null}
              disabledReason={blocked}
              tooltip="Replaces the untouched starter scripts with the newer ones"
              onClick={() => void run('all', update, false)}
            >
              Update
            </Button>
          }
        />
      ) : null}
      {editedBanners}
    </div>
  );
}
