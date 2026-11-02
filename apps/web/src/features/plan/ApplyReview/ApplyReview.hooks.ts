import { usePlanCameraStore } from '@/state/use-plan-camera-store';

/**
 * The Apply review: the exact change to `layout.json` the core worked out,
 * Apply with the keys it was shown with (refused when either file changed
 * since), and Review again after a refusal.
 */
export function useApplyReview() {
  const ask = usePlanCameraStore(s => s.apply);
  const confirm = usePlanCameraStore(s => s.confirmApply);
  const again = usePlanCameraStore(s => s.reviewAgain);
  const close = usePlanCameraStore(s => s.closeApply);
  const review = ask?.review ?? null;
  const problems = (review?.problems ?? []).filter(p => !p.startsWith('Nothing to apply'));
  const d = review?.diff;
  const nothing = Boolean(d && d.added + d.removed + d.moved + d.changed === 0);
  return {
    review,
    error: ask?.error ?? null,
    applying: ask?.applying ?? false,
    problems,
    /** Why Apply is off in the review, or null. */
    off: ask?.error
      ? 'Review again first'
      : problems.length
        ? problems[0]!
        : nothing
          ? 'layout.json already has this order and these settings'
          : null,
    apply: () => void confirm(),
    again: () => void again(),
    close,
  };
}
