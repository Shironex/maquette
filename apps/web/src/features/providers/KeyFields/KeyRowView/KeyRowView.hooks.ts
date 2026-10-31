import { useState } from 'react';

export interface KeyResult {
  ok: boolean;
  message: string;
}

export interface KeyCalls<R extends KeyResult> {
  /** The saved key: whether there is one and its last four characters. */
  status: { saved: boolean; tail: string | null } | undefined;
  busy: boolean;
  /** Nothing is known yet: the read is waiting, maybe on a Keychain prompt. */
  reading?: boolean;
  save: (key: string) => Promise<boolean>;
  remove: () => Promise<void>;
  /** Tests the pasted key, or the saved one with null. */
  test: (key: string | null) => Promise<R | null>;
}

export type KeyDraft = ReturnType<typeof useKeyDraft>;

/** A saved key, the draft for a new one and the last test result. */
export function useKeyDraft<R extends KeyResult>({
  status,
  busy,
  reading = false,
  save,
  remove,
  test,
}: KeyCalls<R>) {
  const [draft, setDraft] = useState('');
  const [editing, setEditing] = useState(false);
  const [result, setResult] = useState<KeyResult | null>(null);
  const saved = Boolean(status?.saved);
  const showInput = !reading && (!saved || editing);

  const onSave = async () => {
    if (!(await save(draft))) return;
    setDraft('');
    setEditing(false);
    setResult(null);
  };
  const onTest = async () => setResult(await test(showInput && draft ? draft : null));
  // A test result speaks for the key it tested, so it goes with that key.
  const onRemove = async () => {
    setResult(null);
    await remove();
  };
  const replace = (on: boolean) => {
    setEditing(on);
    setDraft('');
    setResult(null);
  };

  return {
    status,
    busy,
    reading,
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
  };
}
