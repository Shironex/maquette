import type { LogLine } from '@maquette/ui';
import { classifyLog, sizeChange, type LogKind } from '@/state/log-lines';
import type { LogRow } from '@/state/pipeline';

/** The console level each kind of line takes; plain and tagged lines keep the base color. */
const LEVEL: Partial<Record<LogKind, LogLine['level']>> = {
  error: 'error',
  warning: 'warn',
  waiting: 'warn',
  milestone: 'ok',
  result: 'info',
};

/** A result line with the size it ends at in bold. */
function Result({ text }: { text: string }) {
  const change = sizeChange(text);
  if (!change) return <>{text}</>;
  return (
    <>
      {change.before}
      {change.between}
      <strong className="font-semibold">{change.size}</strong>
      {change.after}
    </>
  );
}

/** A log row as the console draws it. */
export function styledLine(row: LogRow): LogLine & { noise: boolean } {
  const { kind, tag, body } = classifyLog(row.text, row.level);
  const base = { t: row.t, text: row.text, noise: kind === 'noise' };
  if (kind === 'noise') return { ...base, dim: true };
  const level = LEVEL[kind];
  const rest = kind === 'result' || kind === 'tagged' ? <Result text={body} /> : body;
  if (!tag) return { ...base, level, node: kind === 'result' ? rest : undefined };
  return {
    ...base,
    level,
    node: (
      <>
        <span className={kind === 'tagged' ? 'text-clay-text' : undefined}>[{tag}]</span> {rest}
      </>
    ),
  };
}
