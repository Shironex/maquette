import { Button, EmptyState, Tabs, TopBar, Loader } from '@maquette/ui';
import { ProjectTopBar } from '@/features/common/ProjectTopBar';
import { CompareBoard } from '../CompareBoard';
import { DeviceVerdict } from '../DeviceVerdict';
import { DevicesBoard } from '../DevicesBoard';
import { FrameStatus } from '../FrameStatus';
import { PageFrame } from '../PageFrame';
import { PageReadouts } from '../PageReadouts';
import { PageSource } from '../PageSource';
import { PageToolbar } from '../PageToolbar';
import { ServerControls } from '../ServerControls';
import { ShipTable } from '../ShipTable';
import { StopScrubber } from '../StopScrubber';
import { usePageView } from './PageView.hooks';

const TABS = [
  { id: 'preview', label: 'Preview' },
  { id: 'devices', label: 'Devices' },
];

/**
 * The project's site, or an HTML page concept, live in the app: the page
 * at a device's width with the camera stop scrubber, the numbers it runs
 * at, and the board that puts the current ship beside this run.
 */
export default function PageView() {
  const v = usePageView();
  if (!v.project) {
    return (
      <>
        <TopBar lead={<h1 className="text-panel-title">Page</h1>} />
        <main className="grid flex-1 place-items-center">
          <EmptyState
            icon="browser"
            title="No project yet"
            body="Create a project in Settings first."
          />
        </main>
      </>
    );
  }
  if (!v.view || !v.state) {
    return (
      <>
        <ProjectTopBar />
        <main className="grid flex-1 place-items-center" aria-busy={v.loading}>
          {v.error ? (
            <EmptyState
              icon="warning-circle"
              title="The page settings could not be read"
              body={v.error}
              action={
                <Button
                  icon="arrow-clockwise"
                  tooltip="Reads the page settings again"
                  onClick={v.retry}
                >
                  Try again
                </Button>
              }
            />
          ) : (
            <Loader label="Reading the page..." />
          )}
        </main>
      </>
    );
  }
  const status = v.state.kind === 'running' ? null : v.state;
  return (
    <>
      <ProjectTopBar
        extra={
          <Button
            icon="arrow-square-out"
            disabled={!v.canOpen}
            disabledReason="Opens once the site's dev server runs"
            tooltip="Opens this page on the dev server in your browser"
            onClick={v.openInBrowser}
          >
            Open in browser
          </Button>
        }
      />
      <main className="flex min-h-0 flex-1">
        <section aria-label="Page" className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 px-5 py-4">
          <div className="border-b border-line">
            <Tabs tabs={TABS} value={v.tab} onChange={v.setTab} />
          </div>
          <PageToolbar />
          {v.board === 'compare' ? (
            <CompareBoard />
          ) : v.board === 'devices' ? (
            <DevicesBoard />
          ) : (
            <PageFrame
              id="main"
              src={v.source.src}
              address={v.source.address}
              badge={v.badge}
              device={v.device}
              showStop={v.showStops}
              label={v.label}
              actions={v.concept ? null : <ServerControls />}
              className="flex-1"
            >
              {status ? <FrameStatus state={status} site={v.site} command={v.command} /> : null}
            </PageFrame>
          )}
          {v.board === 'compare' ? null : <StopScrubber />}
        </section>
        <aside
          aria-label={v.compareSide ? 'Device verdict' : 'Readouts and source'}
          className="flex w-[344px] flex-none flex-col gap-3.5 overflow-y-auto border-l border-line bg-panel p-4"
        >
          {v.compareSide ? (
            <DeviceVerdict />
          ) : (
            <>
              <PageReadouts />
              <div className="h-px flex-none bg-line" />
              <ShipTable />
              <div className="h-px flex-none bg-line" />
              <PageSource />
            </>
          )}
        </aside>
      </main>
    </>
  );
}
