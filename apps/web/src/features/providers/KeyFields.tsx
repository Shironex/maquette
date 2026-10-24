/**
 * Settings > 3D providers: the Tripo and Meshy API keys. They live in the
 * OS keychain; the UI only ever sees the last four characters.
 */
import { useState } from 'react';
import { Button, Icon, TextField, cn } from '@maquette/ui';
import type { KeyTest, ProviderId } from '@/ipc/client';
import { PLATFORM } from '@/ipc/runtime';
import { SERVICE_LABEL } from '@/state/providers';
import { useProvidersStore } from '@/state/providers-store';

const WHERE: Record<ProviderId, string> = {
  tripo: 'platform.tripo3d.ai, API keys',
  meshy: 'meshy.ai, Settings, API',
};

const KEYCHAIN = PLATFORM === 'windows' ? 'Credential Manager' : 'Keychain';

export function KeyFields() {
  return (
    <div className="flex flex-col gap-2">
      {(['tripo', 'meshy'] as const).map(service => (
        <KeyRow key={service} service={service} />
      ))}
      <p className="text-meta text-fg-mute">
        Keys stay in the {KEYCHAIN}, never in maquette.json, a project file or a log. Test key uses
        the free balance call, so it spends nothing.
      </p>
    </div>
  );
}

function KeyRow({ service }: { service: ProviderId }) {
  const status = useProvidersStore(s => s.keys.find(k => k.service === service));
  const busy = useProvidersStore(s => s.keyBusy === service);
  const save = useProvidersStore(s => s.saveKey);
  const remove = useProvidersStore(s => s.removeKey);
  const test = useProvidersStore(s => s.testKey);
  const [draft, setDraft] = useState('');
  const [editing, setEditing] = useState(false);
  const [result, setResult] = useState<KeyTest | null>(null);
  const label = SERVICE_LABEL[service];
  const saved = Boolean(status?.saved);
  const showInput = !saved || editing;

  const onSave = async () => {
    if (await save(service, draft)) {
      setDraft('');
      setEditing(false);
      setResult(null);
    }
  };
  const onTest = async () => setResult(await test(service, showInput && draft ? draft : null));

  return (
    <div className="flex flex-col gap-2 rounded-[8px] border border-line bg-raised px-3 py-2.5">
      <div className="flex items-center gap-2.5">
        <span className="font-semibold">{label}</span>
        {saved ? (
          <>
            <code className="flex min-w-0 items-center gap-1.5 truncate font-mono text-[12.5px] text-fg-dim">
              <Icon name="lock-simple" size={13} className="flex-none text-fg-mute" />
              {'\u2022'.repeat(8)}
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
            onClick={() => void onSave()}
          >
            Save
          </Button>
          {editing ? (
            <Button variant="ghost" onClick={() => setEditing(false)}>
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
            onClick={() => void onTest()}
          >
            Test key
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setEditing(true)}>
            Replace
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={busy}
            onClick={() => void remove(service)}
            className="ml-auto text-fg-mute"
          >
            Remove
          </Button>
        </div>
      )}
      {showInput && !draft.trim() ? (
        <p className="text-meta text-fg-mute">From {WHERE[service]}.</p>
      ) : null}
      {showInput && draft.trim() ? (
        <button
          type="button"
          className="self-start text-meta text-fg-mute underline-offset-2 hover:text-fg hover:underline"
          onClick={() => void onTest()}
        >
          Test this key before saving
        </button>
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
