import { useState } from 'react';
import { copyText } from '@/features/config/copyText';

/** Copies the prompt, and says for a moment that it did or why it could not. */
export function useAgentPrompt(prompt: string) {
  const [copied, setCopied] = useState<'copied' | 'failed' | null>(null);
  return {
    copied,
    copy: () =>
      void copyText(prompt).then(ok => {
        setCopied(ok ? 'copied' : 'failed');
        setTimeout(() => setCopied(null), 2000);
      }),
  };
}
