import { Button } from '@maquette/ui';
import { usePreviewSite } from './PreviewSiteButton.hooks';

/** Opens the project's site on its own dev server; deploying stays the person's step. */
export default function PreviewSiteButton() {
  const { hasSite, starting, preview } = usePreviewSite();
  return (
    <Button
      icon="arrow-square-out"
      loading={starting}
      disabled={!hasSite}
      disabledReason="Pick the site repo in Settings > Project first."
      tooltip="Starts the site's dev server and opens it. Nothing is deployed."
      onClick={() => void preview()}
    >
      Preview on site
    </Button>
  );
}
