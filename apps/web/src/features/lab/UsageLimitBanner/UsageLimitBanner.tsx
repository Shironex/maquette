import { Button, InlineBanner } from '@maquette/ui';
import type { LabView } from '@/ipc/client';
import { useUsageLimitBanner } from './UsageLimitBanner.hooks';

export interface UsageLimitBannerProps {
  view: LabView;
}

/** Above the results while Codex's usage limit holds jobs: until when, and Limit lifted, retry now. */
export default function UsageLimitBanner({ view }: UsageLimitBannerProps) {
  const { banner, answer, lifting, retryNow } = useUsageLimitBanner(view);
  if (!banner) return null;
  return (
    <InlineBanner
      tone="warning"
      className="mx-[22px] mt-[18px] flex-none"
      title={banner.title}
      detail={
        <>
          {banner.detail}
          {answer ? <span className="mt-1 block text-fg">{answer}</span> : null}
        </>
      }
      actions={
        <Button
          size="sm"
          variant="secondary"
          icon="arrow-clockwise"
          loading={lifting}
          tooltip="Reads the Codex usage again and runs the waiting images now if the limit has lifted"
          onClick={retryNow}
        >
          Limit lifted, retry now
        </Button>
      }
    />
  );
}
