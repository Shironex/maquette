import { Button, Icon, IconButton } from '@maquette/ui';
import { maskToken } from '@/state/settings-form';
import { useTokenRow } from './TokenRow.hooks';

/** The bearer token, masked until asked, once the server has made one. */
export default function TokenRow() {
  const { token, shown, toggleShown, copy, renew } = useTokenRow();
  if (!token) return null;
  return (
    <div className="flex items-center gap-2.5 rounded-control border border-line bg-app px-3 py-2">
      <Icon name="lock-simple" size={14} className="text-fg-mute" />
      <span className="text-meta text-fg-mute">Bearer token</span>
      <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg-dim">
        {shown ? token : maskToken(token)}
      </code>
      <IconButton
        icon="eye"
        label={shown ? 'Hide the token' : 'Show the token'}
        aria-pressed={shown}
        size="sm"
        variant="ghost"
        onClick={toggleShown}
      />
      <IconButton icon="copy" label="Copy the token" size="sm" variant="ghost" onClick={copy} />
      <Button
        size="sm"
        variant="ghost"
        tooltip="Makes a new token; agents set up with the old one stop working"
        onClick={() => void renew()}
      >
        New token
      </Button>
    </div>
  );
}
