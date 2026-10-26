import type { ReactNode } from 'react';
import { Button, EmptyState, Icon, TopBar } from '@maquette/ui';
import { ProjectTopBar } from '@/features/common/ProjectTopBar';
import { TimelineEmpty } from '../TimelineEmpty';
import { TimelinePanel } from '../TimelinePanel';
import { TimelineTracks } from '../TimelineTracks';
import { useTimelineView } from './TimelineView.hooks';

export interface TimelineViewProps {
  /** The project's page, which the playhead scrolls (the shell passes the Page view's frame). */
  preview: ReactNode;
}

/**
 * The scroll timeline: the page from top to bottom with the camera stops
 * as columns, a lane per prop with states and per explode track, and the
 * page itself above it, scrolled to the playhead.
 */
export default function TimelineView({ preview }: TimelineViewProps) {
  const v = useTimelineView();
  const bar = (
    <ProjectTopBar
      extra={
        <Button icon="browser" onClick={v.openPage}>
          Preview on page
        </Button>
      }
    />
  );
  if (!v.project) {
    return (
      <>
        <TopBar lead={<h1 className="text-panel-title">Timeline</h1>} />
        <main className="grid flex-1 place-items-center">
          <EmptyState
            icon="film-strip"
            title="No project yet"
            body="Create a project in Settings first."
          />
        </main>
      </>
    );
  }
  if (!v.view) {
    return (
      <>
        {bar}
        <main className="grid flex-1 place-items-center" aria-busy={v.loading}>
          {v.error ? (
            <EmptyState
              icon="warning-circle"
              title="The timeline could not be read"
              body={v.error}
              action={
                <Button icon="arrow-clockwise" onClick={v.retry}>
                  Try again
                </Button>
              }
            />
          ) : null}
        </main>
      </>
    );
  }
  if (v.view.stops.length === 0) {
    return (
      <>
        {bar}
        <main className="flex min-h-0 flex-1">
          <TimelineEmpty />
        </main>
      </>
    );
  }
  return (
    <>
      {bar}
      <main className="flex min-h-0 flex-1">
        <section
          aria-label="Timeline"
          className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-y-auto px-5 py-4"
        >
          <div className="flex items-center gap-2.5">
            <h1 className="text-screen-title">Scroll timeline</h1>
            <span className="flex-1" />
            <span className="text-meta text-fg-mute" aria-live="polite">
              {v.saveLine}
            </span>
          </div>
          <div className="relative min-h-[240px] flex-1">
            {preview}
            <span className="pointer-events-none absolute bottom-3 left-3 inline-flex h-[22px] items-center gap-[5px] rounded-control bg-[rgba(28,27,25,0.78)] px-2 text-[11.5px] font-medium text-fg">
              <Icon name="video-camera" size={12} className="text-fg-dim" />
              {v.place}
            </span>
            <Button
              size="sm"
              icon="browser"
              onClick={v.openPage}
              className="absolute right-3 bottom-3"
            >
              Open on the page
            </Button>
          </div>
          <TimelineTracks />
        </section>
        <TimelinePanel />
      </main>
    </>
  );
}
