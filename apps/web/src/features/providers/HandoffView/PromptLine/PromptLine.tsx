import { Button } from '@maquette/ui';
import { useCopy } from '../CopyButton';

export interface PromptLineProps {
  prompt: string;
}

/** The text for the site's prompt box, one click to copy. */
export default function PromptLine({ prompt }: PromptLineProps) {
  const copy = useCopy(prompt, 'The prompt');
  return (
    <div className="flex items-center gap-2 rounded-control border border-line bg-raised px-2.5 py-1.5">
      <p className="min-w-0 flex-1 truncate text-[12.5px] text-fg-dim" title={prompt}>
        {prompt}
      </p>
      <Button
        size="sm"
        variant="secondary"
        icon="copy"
        tooltip="Copies the prompt to paste on the site"
        onClick={copy}
      >
        Copy prompt
      </Button>
    </div>
  );
}
