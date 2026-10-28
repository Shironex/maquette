import { useRef } from 'react';
import { useTopEscape } from '@/lib/useTopEscape';
import { reviewCounts, stepIndex } from '@/state/retarget';
import { useRetargetStore } from '@/state/use-retarget-store';

/** The steps' shell: which step shows, what the main button does and says. */
export function useRetargetDialog() {
  const sheet = useRetargetStore(s => s.sheet);
  const busy = useRetargetStore(s => s.busy);
  const close = useRetargetStore(s => s.close);
  const next = useRetargetStore(s => s.next);
  const back = useRetargetStore(s => s.back);
  const goTo = useRetargetStore(s => s.goTo);
  const bake = useRetargetStore(s => s.bake);
  const ref = useRef<HTMLDivElement>(null);
  useTopEscape(ref, close);
  if (!sheet) return null;

  const name = sheet.plan?.name ?? 'a model';
  const chosen = sheet.chosen.length;
  const counts = sheet.draft ? reviewCounts(sheet.draft.review.clips) : null;
  const baking = counts ? counts.good + counts.look : 0;
  const primary = {
    choose: {
      label: 'Next: bone map',
      onClick: next,
      disabled: !sheet.asset || chosen === 0,
      reason: !sheet.asset ? 'Pick a rigged model first' : 'Choose at least one clip',
    },
    map: { label: 'Next: clip settings', onClick: next, disabled: false, reason: '' },
    settings: {
      label: busy ? 'Baking in Blender' : `Retarget ${chosen} ${chosen === 1 ? 'clip' : 'clips'}`,
      onClick: next,
      disabled: busy || chosen === 0,
      reason: busy ? 'Blender is baking the clips' : 'Choose at least one clip',
    },
    review: {
      label: `Bake ${baking} ${baking === 1 ? 'clip' : 'clips'} into v${(sheet.draft?.fromVersion ?? 0) + 1}`,
      onClick: bake,
      disabled: busy || baking === 0,
      reason: busy ? 'Adding the version' : 'Every clip failed in review',
    },
  }[sheet.step];
  const subtitle = {
    choose:
      'Opened from Motion or Use on model in the Animations library. Nothing is baked until Review.',
    map: `${chosen} ${chosen === 1 ? 'clip' : 'clips'} chosen. One map covers every clip of a rig; it is kept for this model.`,
    settings:
      'Names, frames, loop, motion and speed are for this model only, never saved back to the library.',
    review: `Baked headless in Blender on a copy of v${sheet.draft?.fromVersion ?? ''}. Nothing is saved until you bake.`,
  }[sheet.step];
  return {
    sheet,
    busy,
    ref,
    title:
      sheet.step === 'review' ? `Review the retarget on ${name}` : `Add library clips to ${name}`,
    subtitle,
    at: stepIndex(sheet.step),
    primary,
    close,
    back,
    goTo,
    /** Steps after Choose need a model and clips. */
    canJump: Boolean(sheet.asset && chosen),
  };
}
