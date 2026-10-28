import { Tooltip, cn } from '@maquette/ui';
import type { CodexUsage } from '@/ipc/client';
import type { UsageTone } from '@/state/codex-usage';
import { useCodexUsageItem } from './CodexUsageItem.hooks';

export interface CodexUsageItemProps {
  usage: CodexUsage;
}

const DOT: Record<UsageTone, string> = {
  ok: 'bg-sage',
  warn: 'bg-amber',
  limit: 'bg-danger',
};

const TEXT: Record<UsageTone, string> = {
  ok: '',
  warn: 'text-amber',
  limit: 'text-danger',
};

/** The status bar's Codex usage: amber from 80%, red at the limit; the tooltip has the reset times. */
export default function CodexUsageItem({ usage }: CodexUsageItemProps) {
  const { label, detail, tone, reading, read } = useCodexUsageItem(usage);
  return (
    <Tooltip label="Codex usage, click to read again" detail={detail} wrap>
      <button
        type="button"
        onClick={read}
        aria-busy={reading}
        data-tone={tone}
        className={cn(
          '-mx-1.5 flex h-6 cursor-pointer items-center gap-1.5 rounded-control px-1.5 hover:bg-hover hover:text-fg',
          TEXT[tone]
        )}
      >
        <span aria-hidden="true" className={cn('size-[7px] flex-none rounded-full', DOT[tone])} />
        {label}
      </button>
    </Tooltip>
  );
}
