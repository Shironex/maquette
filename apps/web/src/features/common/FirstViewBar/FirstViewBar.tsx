import { BudgetBar } from '@maquette/ui';
import { useFirstViewBar } from './FirstViewBar.hooks';
import type { FirstViewBarProps } from './FirstViewBar.types';

const oneDecimal = (n: number) => n.toFixed(1);

/** What the first camera stop loads against its own budget, for the Pipeline view. */
export default function FirstViewBar({ className }: FirstViewBarProps) {
  const bar = useFirstViewBar();
  if (!bar) return null;
  return (
    <div className={className} title={bar.title}>
      <BudgetBar
        label={bar.label}
        value={bar.value}
        target={bar.target}
        unit="MB"
        format={oneDecimal}
        showNote={false}
      />
    </div>
  );
}
