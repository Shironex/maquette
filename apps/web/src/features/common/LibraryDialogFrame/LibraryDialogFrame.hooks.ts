import { useRef, type FormEvent } from 'react';
import { useTopEscape } from '@/lib/useTopEscape';

/** Escape closes while this is the top dialog; the form submits without a page load. */
export function useLibraryDialogFrame(onClose: () => void, onSubmit: () => void) {
  const ref = useRef<HTMLFormElement>(null);
  useTopEscape(ref, onClose);
  const submit = (e: FormEvent) => {
    e.preventDefault();
    onSubmit();
  };
  return { ref, submit };
}
