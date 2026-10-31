import type { CleanGround } from '@/ipc/client';

export interface KeptRemoved {
  kept: string[];
  removed: string[];
}

/** What the copy keeps and what it clears, as the copy's options set them. */
export function keptAndRemoved(ground: CleanGround, pad: boolean, extend: boolean): KeptRemoved {
  const kept = ['The object, as drawn', 'The original file, moved to refs/originals'];
  const removed = [
    'Clutter and other things around the object',
    ground === 'transparent' ? 'The background, left transparent' : 'The background, studio grey',
  ];
  if (pad) kept.push('Room around the object on every side, a square frame');
  if (extend) removed.push('The cut off edge, drawn in where the frame stopped');
  return { kept, removed };
}
