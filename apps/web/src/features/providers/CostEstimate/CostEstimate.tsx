import { HelpMark, cn } from '@maquette/ui';
import { estimateText } from '@/state/credits';

export interface CostEstimateProps {
  /** Credits from the price lists; null shows nothing. */
  credits: number | null;
  className?: string;
}

/** "About 30 credits" with a mark that says where the number comes from. */
export default function CostEstimate({ credits, className }: CostEstimateProps) {
  const text = estimateText(credits);
  if (!text) return null;
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 text-meta text-fg-dim tabular-nums',
        className
      )}
    >
      <span className="font-medium text-fg">{text}</span>
      <HelpMark
        label="About this estimate"
        tip="From the provider's published price list for these settings. The provider reports what the task really cost once it finishes, and the Credits view keeps both."
      />
    </span>
  );
}
