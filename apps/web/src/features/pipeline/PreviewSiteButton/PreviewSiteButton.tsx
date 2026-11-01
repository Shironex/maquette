import { Button } from '@maquette/ui';
import { StopServerButton } from '@/features/common/StopServerButton';
import { usePreviewSite } from './PreviewSiteButton.hooks';

/**
 * Opens the project's site on its own dev server; deploying stays the
 * person's step. While the server runs it splits into Open preview and Stop.
 */
export default function PreviewSiteButton() {
  const { id, hasSite, starting, running, openTooltip, preview, open } = usePreviewSite();
  const split = running || starting;
  return (
    <div role="group" aria-label="Site preview" className="flex items-center">
      {running ? (
        <Button
          icon="arrow-square-out"
          tooltip={openTooltip}
          className="rounded-r-none"
          onClick={open}
        >
          Open preview
        </Button>
      ) : (
        <Button
          icon="arrow-square-out"
          loading={starting}
          disabled={!hasSite}
          disabledReason="Pick the site repo in Settings > Project first."
          tooltip={
            starting
              ? "Starting the site's dev server; it opens once the site answers"
              : "Starts the site's dev server and opens it. Nothing is deployed."
          }
          className={split ? 'rounded-r-none' : undefined}
          onClick={() => void preview()}
        >
          <span className="@max-[900px]:sr-only">Preview on site</span>
        </Button>
      )}
      {split ? <StopServerButton project={id} compact className="-ml-px rounded-l-none" /> : null}
    </div>
  );
}
