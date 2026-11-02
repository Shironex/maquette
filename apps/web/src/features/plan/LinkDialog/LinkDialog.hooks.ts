import { useEffect, useId, useState } from 'react';
import { trimText } from '@/state/plan-text';
import { webAddress } from '@/state/plans';

/** The address and title of a new link node; Add hands them back once the address reads. */
export function useLinkDialog(onAdd: (url: string, title: string) => void) {
  const [url, setUrl] = useState('');
  const [title, setTitle] = useState('');
  const [touched, setTouched] = useState(false);
  const field = useId();
  useEffect(() => {
    document.getElementById(field)?.focus();
  }, [field]);
  const address = webAddress(url);
  return {
    url,
    setUrl,
    title,
    setTitle,
    field,
    canAdd: address !== null,
    note:
      touched && trimText(url) && !address
        ? { tone: 'warning' as const, text: 'Type a web address, such as example.com/article.' }
        : undefined,
    touch: () => setTouched(true),
    submit: () => {
      setTouched(true);
      if (address) onAdd(address, trimText(title));
    },
  };
}
