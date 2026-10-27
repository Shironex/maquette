import { StopServerButton } from '@/features/common/StopServerButton';
import { useServerControls } from './ServerControls.hooks';

/**
 * The end of the main frame's address bar, where the server's state shows:
 * who started it, when not the Page view, and Stop. Stop here, in the
 * Pipeline or in the status bar ends the same server.
 */
export default function ServerControls() {
  const { project, owner } = useServerControls();
  return (
    <>
      {owner ? (
        <span className="flex-none truncate font-sans text-[11.5px] text-fg-mute">{owner}</span>
      ) : null}
      <StopServerButton project={project} size="sm" />
    </>
  );
}
