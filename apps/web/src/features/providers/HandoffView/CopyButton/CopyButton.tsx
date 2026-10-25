import { IconButton } from '@maquette/ui';
import { useCopy } from './CopyButton.hooks';

export interface CopyButtonProps {
  text: string;
  what: string;
}

export default function CopyButton({ text, what }: CopyButtonProps) {
  const copy = useCopy(text, what);
  return <IconButton icon="copy" size="sm" label={`Copy ${what}`} onClick={copy} />;
}
