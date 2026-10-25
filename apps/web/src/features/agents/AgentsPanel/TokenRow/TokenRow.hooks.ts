import { useState } from 'react';
import { copyText } from '@/features/config/copyText';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';

/** The server's token, whether it is shown, and the copy and renew actions. */
export function useTokenRow() {
  const token = useAgentsStore(s => s.status?.token);
  const [shown, setShown] = useState(false);
  const renew = useAgentsStore(s => s.renewToken);
  const notify = useAppStore(s => s.notify);
  const toggleShown = () => setShown(s => !s);
  const copy = () => {
    if (!token) return;
    void copyText(token).then(ok =>
      notify(
        ok
          ? { status: 'done', title: 'Token copied' }
          : { status: 'failed', title: 'Could not copy' }
      )
    );
  };
  return { token, shown, toggleShown, copy, renew };
}
