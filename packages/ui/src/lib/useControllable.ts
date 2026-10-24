import { useCallback, useState } from 'react';

/**
 * State that a parent may own (`value` given) or leave to the component
 * (`defaultValue`). Every kit control with a value goes through this.
 */
export function useControllable<T>(
  value: T | undefined,
  defaultValue: T,
  onChange?: (next: T) => void
): [T, (next: T) => void] {
  const [inner, setInner] = useState(defaultValue);
  const controlled = value !== undefined;
  const current = controlled ? value : inner;
  const set = useCallback(
    (next: T) => {
      if (!controlled) setInner(next);
      onChange?.(next);
    },
    [controlled, onChange]
  );
  return [current, set];
}
