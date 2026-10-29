/**
 * One saved API key: its last four characters and Test, Replace and
 * Remove, or a field to paste one. The 3D providers' keys and OpenAI's
 * share it; each passes its own key calls through [`useKeyDraft`].
 */
import { Button, Icon, TextField, Tooltip, cn } from '@maquette/ui';
import { KEYCHAIN } from '../keychain';
import type { KeyDraft } from './KeyRowView.hooks';

export interface KeyRowViewProps extends KeyDraft {
  /** "Tripo", "OpenAI". */
  label: string;
  /** Where the key is made, for example "meshy.ai, Settings, API". */
  where: string;
  /** What Test key does, for its tooltip. */
  testTip: string;
}

export default function KeyRowView({
  label,
  where,
  testTip,
  status,
  busy,
  draft,
  setDraft,
  editing,
  result,
  saved,
  showInput,
  onSave,
  onTest,
  onRemove,
  replace,
}: KeyRowViewProps) {
  return (
    <div className="flex flex-col gap-2 rounded-[8px] border border-line bg-raised px-3 py-2.5">
      <div className="flex items-center gap-2.5">
        <span className="font-semibold">{label}</span>
        {saved ? (
          <>
            <code className="flex min-w-0 items-center gap-1.5 truncate font-mono text-[12.5px] text-fg-dim">
              <Icon name="lock-simple" size={13} className="flex-none text-fg-mute" />
              {'•'.repeat(8)}
              {status?.tail}
            </code>
            <span className="ml-auto flex-none text-meta text-fg-mute">in the {KEYCHAIN}</span>
          </>
        ) : (
          <span className="ml-auto text-meta text-fg-mute">No key yet</span>
        )}
      </div>
      {showInput ? (
        <div className="flex items-center gap-2">
          <TextField
            aria-label={`${label} API key`}
            type="password"
            mono
            autoComplete="off"
            placeholder={editing ? 'Paste the new key' : 'Paste the API key'}
            value={draft}
            onChange={e => setDraft(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter' && draft.trim()) void onSave();
            }}
            fieldClassName="min-w-0 flex-1"
          />
          <Button
            variant="primary"
            loading={busy}
            disabled={!draft.trim() || busy}
            disabledReason={busy ? 'Working on it' : 'Paste the key first'}
            tooltip="Saves the key on this computer"
            onClick={() => void onSave()}
          >
            Save
          </Button>
          {editing ? (
            <Button variant="ghost" tooltip="Keeps the key you have" onClick={() => replace(false)}>
              Keep the old one
            </Button>
          ) : null}
        </div>
      ) : (
        <div className="flex items-center gap-1.5">
          <Button
            size="sm"
            icon="plugs-connected"
            loading={busy}
            disabled={busy}
            tooltip={testTip}
            onClick={() => void onTest()}
          >
            Test key
          </Button>
          <Button
            size="sm"
            variant="ghost"
            tooltip="Paste a new key in place of the saved one"
            onClick={() => replace(true)}
          >
            Replace
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={busy}
            tooltip="Deletes the saved key from this computer"
            onClick={() => void onRemove()}
            className="ml-auto text-fg-mute"
          >
            Remove
          </Button>
        </div>
      )}
      {showInput && !draft.trim() ? <p className="text-meta text-fg-mute">From {where}.</p> : null}
      {showInput && draft.trim() ? (
        <Tooltip
          label="Test this key"
          detail="Asks the service whether it accepts the key; nothing is saved"
        >
          <button
            type="button"
            className="self-start text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline"
            onClick={() => void onTest()}
          >
            Test this key before saving
          </button>
        </Tooltip>
      ) : null}
      {result ? (
        <p
          role="status"
          className={cn(
            'flex items-center gap-1.5 text-meta',
            result.ok ? 'text-sage' : 'text-danger'
          )}
        >
          <Icon name={result.ok ? 'check-circle' : 'warning-circle'} size={14} />
          {result.message}
        </p>
      ) : null}
    </div>
  );
}
