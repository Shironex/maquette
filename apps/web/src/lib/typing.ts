/** True when a key press is meant for a text field, not a view's shortcuts. */
export function typing(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return Boolean(target.closest('input, textarea, select, [contenteditable="true"]'));
}
