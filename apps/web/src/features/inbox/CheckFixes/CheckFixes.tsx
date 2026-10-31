import { Button } from '@maquette/ui';
import type { Check, InboxItem } from '@/ipc/client';
import { useCheckFixes, useProviderSplit, useRigDoctorFix } from './CheckFixes.hooks';

export interface CheckFixesProps {
  item: InboxItem;
  check: Check;
}

/** The one click fix-ups a check offers; a pick goes into the item's draft. */
export default function CheckFixes({ item, check }: CheckFixesProps) {
  const buttons = useCheckFixes(item, check).map(({ fix, added, text, toggle }) => (
    <Button
      key={fix.op}
      size="sm"
      variant={added ? 'ghost' : 'secondary'}
      icon={added ? 'check' : 'wrench'}
      aria-pressed={added}
      tooltip={
        added
          ? 'In the fix-ups below. Click to take it out.'
          : 'Adds it to the fix-ups below. They run headless on a copy; the file never changes.'
      }
      onClick={toggle}
    >
      {added ? `${text} added` : text}
    </Button>
  ));
  const openSplit = useProviderSplit(item, check);
  const split = openSplit ? (
    <Button
      size="sm"
      variant="secondary"
      icon="stack"
      tooltip="Splits this model on Tripo or Meshy; the split comes back as another Inbox model"
      onClick={openSplit}
    >
      Split on Tripo or Meshy...
    </Button>
  ) : null;
  const openDoctor = useRigDoctorFix(item, check);
  const doctor = openDoctor ? (
    <Button
      size="sm"
      variant="secondary"
      icon="wrench"
      tooltip="Reads the rig in seconds; the pose test and fix wait for your click, on a copy"
      onClick={openDoctor}
    >
      Fix in rig doctor
    </Button>
  ) : null;
  return (
    <>
      {buttons}
      {split}
      {doctor}
    </>
  );
}
