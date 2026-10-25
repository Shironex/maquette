import { STEPS } from '@/state/onboarding';
import { HEADINGS } from './headings';
import type { StepHeadingProps } from './StepHeading.types';

export default function StepHeading({ step, name }: StepHeadingProps) {
  const heading = HEADINGS[step];
  if (!heading) return null;
  const number = step + 1;
  const title = heading.title.replace('{name}', name.trim() || 'Your project');
  return (
    <>
      <p className="font-mono text-[12px] leading-4 font-medium tracking-[0.02em] text-clay-text">
        Step {number} of {STEPS.length}
      </p>
      <h1 className="mt-2.5 text-onboarding-title">{title}</h1>
      <p className="mt-2.5 max-w-[600px] text-[15px] leading-[23px] text-fg-dim">{heading.lead}</p>
    </>
  );
}
