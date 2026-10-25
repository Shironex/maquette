import { useEffect, useState } from 'react';

interface NumberTextOptions {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  format: (value: number) => string;
  parse: (text: string) => number | null;
}

/**
 * The field's free text while focused. It follows the value while not being
 * edited and is committed, clamped, on blur; text it cannot read reverts.
 */
export function useNumberText({ value, onChange, min, max, format, parse }: NumberTextOptions) {
  const [text, setText] = useState(format(value));
  const [editing, setEditing] = useState(false);

  useEffect(() => {
    if (!editing) setText(format(value));
  }, [value, editing, format]);

  function commit() {
    setEditing(false);
    const parsed = parse(text);
    if (parsed === null) {
      setText(format(value));
      return;
    }
    let next = parsed;
    if (min !== undefined) next = Math.max(min, next);
    if (max !== undefined) next = Math.min(max, next);
    setText(format(next));
    if (next !== value) onChange(next);
  }

  return { text, setText, setEditing, commit };
}
