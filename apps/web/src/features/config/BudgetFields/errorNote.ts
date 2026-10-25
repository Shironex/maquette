export function errorNote(text?: string) {
  return text ? { tone: 'missing' as const, text } : undefined;
}
