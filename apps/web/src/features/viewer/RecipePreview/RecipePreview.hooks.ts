import { useMemo } from 'react';
import { fileUrl } from '@/ipc/files';
import type { RecipePreviewProps } from './RecipePreview';

/**
 * What the preview plays: the base clip (or the rest pose), or every clip
 * of a joined recipe with their rates and timeline once each has a file.
 * `files` names the clip files, so a new set of them is a new stage.
 */
export function useRecipePreview(clip: RecipePreviewProps['clip']) {
  const joined = clip && typeof clip === 'object' && 'joined' in clip ? clip : null;
  const single = clip && typeof clip === 'object' && !('joined' in clip) ? clip : null;
  const rest = clip === 'rest';
  const play = useMemo(() => {
    if (!joined) return null;
    const urls = joined.joined.map(c => fileUrl(c.file));
    if (!urls.every(Boolean)) return null;
    return {
      urls: urls as string[],
      fps: joined.joined.map(c => c.fps),
      timeline: joined.timeline,
    };
  }, [joined]);
  const first = joined?.joined[0];
  const clipUrl = rest ? null : fileUrl(first ? first.file : single?.file);
  return {
    rest,
    clipId: (first ? first.id : single?.id) ?? null,
    clipUrl,
    play,
    playable: !joined || Boolean(play),
    files: play ? [...new Set(play.urls)].join('#') : String(clipUrl),
    base: rest ? 'The rest pose' : joined ? 'The joined clips' : (single?.name ?? 'The base clip'),
  };
}
