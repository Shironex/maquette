import { Button, EmptyState, Icon, InlineBanner, LogConsole } from '@maquette/ui';
import { logLines, useFrameStatus } from './FrameStatus.hooks';
import type { FrameStatusProps } from './FrameStatus.types';

/** What the frame shows until the page answers: the dev server's state and what to do next. */
export default function FrameStatus({ state, site, command }: FrameStatusProps) {
  const { asking, start, pickConcept, showSite, openSettings, projectName } = useFrameStatus();

  switch (state.kind) {
    case 'starting':
      return (
        <div className="flex w-full max-w-[560px] flex-col gap-4 p-8" aria-live="polite">
          <div className="flex items-center gap-3">
            <Icon name="circle-notch" size={20} className="animate-spin text-clay-text" />
            <h2 className="m-0 text-panel-title text-fg">Starting the site&apos;s dev server</h2>
          </div>
          <p className="m-0 text-fg-dim">
            Maquette runs{' '}
            <code className="font-mono text-[12px] text-fg">{state.status.command ?? command}</code>{' '}
            in {site} on a free port, and stops it when you leave the Page view.
          </p>
          <LogConsole
            title="Dev server"
            live
            lines={logLines(state.status.log)}
            height={180}
            autoScroll
          />
        </div>
      );
    case 'failed':
      return (
        <div className="flex w-full max-w-[620px] flex-col gap-3 p-8">
          <InlineBanner
            tone="danger"
            title="The site's dev server did not start"
            detail={state.status.message ?? 'It stopped before it answered.'}
          />
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="primary"
              icon="arrow-clockwise"
              tooltip="Starts the site's dev server again"
              onClick={start}
            >
              Try again
            </Button>
            <Button
              size="sm"
              icon="gear-six"
              tooltip="Opens Settings, where the dev command is set"
              onClick={openSettings}
            >
              Change the dev command
            </Button>
          </div>
          <LogConsole
            title="Dev server"
            lines={logLines(state.status.log)}
            height={200}
            autoScroll
          />
        </div>
      );
    case 'no-site':
      return (
        <EmptyState
          icon="browser"
          title="No site repo yet"
          body="Pick the project's site repo in Settings > Project and the Page view runs its dev server here, with your models in it. Or show an HTML page concept instead."
          action={
            <div className="flex gap-2">
              <Button
                variant="primary"
                icon="gear-six"
                tooltip="Opens Settings, where the site repo is picked"
                onClick={openSettings}
              >
                Open Settings
              </Button>
              <Button
                icon="file-text"
                tooltip="Shows an HTML page from the workspace here, with no dev server"
                onClick={pickConcept}
              >
                Use a page concept
              </Button>
            </div>
          }
        />
      );
    case 'no-command':
      return (
        <EmptyState
          icon="terminal-window"
          title="No dev command found"
          body={state.message}
          action={
            <Button
              icon="gear-six"
              tooltip="Opens Settings, where the dev command is set"
              onClick={openSettings}
            >
              Set the dev command
            </Button>
          }
        />
      );
    case 'busy':
      return (
        <EmptyState
          icon="plugs-connected"
          title={`The dev server serves ${projectName(state.project)}`}
          body="One site dev server runs at a time. Running this project's site stops that one."
          action={
            <Button
              variant="primary"
              icon="play"
              tooltip="Stops the other site's dev server and starts this one"
              onClick={start}
            >
              Run this project&apos;s site
            </Button>
          }
        />
      );
    case 'no-concept':
      return (
        <EmptyState
          icon="file-text"
          title="Pick an HTML page concept"
          body="A page concept is an .html file inside the workspace, for example one the Lab made. It shows here as it is, with no dev server."
          action={
            <div className="flex gap-2">
              <Button
                variant="primary"
                icon="folder-open"
                tooltip="Choose an HTML file in the workspace to show here"
                onClick={pickConcept}
              >
                Pick a page concept
              </Button>
              <Button
                tooltip="Shows the site's dev server instead of a page concept"
                onClick={showSite}
              >
                Show the site instead
              </Button>
            </div>
          }
        />
      );
    default:
      if (asking) {
        return (
          <EmptyState
            icon="terminal-window"
            title="The dev server waits for your approval"
            body={`An agent set this site repo, so ${command ?? 'its dev command'} has not run. Read the folder and the command on the card, then approve or deny it.`}
          />
        );
      }
      return (
        <EmptyState
          icon="browser"
          title="The dev server is stopped"
          body={`Start it to see ${site} here. Maquette runs ${command ?? 'its dev command'} on a free port.`}
          action={
            <Button
              variant="primary"
              icon="play"
              tooltip="Runs the site's dev command on a free port"
              onClick={start}
            >
              Start the dev server
            </Button>
          }
        />
      );
  }
}
