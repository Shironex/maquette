import { Button } from '@maquette/ui';
import type { Check, InboxItem } from '@/ipc/client';
import { useCheckFixes } from './CheckFixes.hooks';

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
  return <>{buttons}</>;
}
