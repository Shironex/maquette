import { parseCount } from '@/features/config/parseCount';
import { formatK } from '@/lib/format';

/** 0 reads as "Adaptive"; the field also takes "auto". */
export function formatFaces(n: number): string {
  return n === 0 ? 'Adaptive' : formatK(n);
}

export function parseFaces(text: string): number | null {
  return /^\s*(adaptive|auto)?\s*$/i.test(text) ? 0 : parseCount(text);
}
