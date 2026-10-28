import { Button, InlineBanner } from '@maquette/ui';
import { fileName } from '@/state/pipeline';
import { useSyncConflictBanner } from './SyncConflictBanner.hooks';

/**
 * Names the conflict copies the sync tool kept of Maquette's data files.
 * Maquette goes on reading the originals and merges nothing: the person
 * compares, keeps what they want in the original and deletes the copy.
 */
export default function SyncConflictBanner() {
  const { shown, conflicts, title, revealLabel, dismiss, show } = useSyncConflictBanner();
  if (!shown) return null;
  const rows = conflicts.map(c => (
    <li key={c.path} className="flex min-w-0 items-center gap-2">
      <span className="min-w-0 flex-1 truncate font-mono text-[12px]" title={c.path}>
        {c.original}
        <span className="text-fg-mute"> has a copy: {fileName(c.path)}</span>
      </span>
      <Button size="sm" variant="ghost" icon="folder-open" onClick={() => void show(c.path)}>
        {revealLabel}
      </Button>
    </li>
  ));
  return (
    <InlineBanner
      tone="warning"
      className="mx-4 mt-3"
      title={title}
      detail={
        <>
          <p className="mb-1.5">
            The workspace changed on two computers at once. Maquette keeps using the original and
            merges nothing: compare the two, keep what you want in the original, then delete the
            copy.
          </p>
          <ul className="m-0 flex list-none flex-col gap-1 p-0">{rows}</ul>
        </>
      }
      actions={
        <Button
          size="sm"
          variant="ghost"
          tooltip="Hides this until the next conflict copy turns up"
          onClick={dismiss}
        >
          Dismiss
        </Button>
      }
    />
  );
}
