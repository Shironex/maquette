import { Button, EmptyState } from '@maquette/ui';
import { ShipSettingsPanel } from './ShipSettingsPanel';
import { SitePlanTable } from './SitePlanTable';
import { SizeReportTable } from './SizeReportTable';
import { useSiteShipView } from './SiteShipView.hooks';

export interface SiteShipViewProps {
  project: string;
  /** Back to the runner. */
  onClose: () => void;
}

/** A website's Ship settings, the dry-run plan and the size report. */
export default function SiteShipView({ project, onClose }: SiteShipViewProps) {
  const ship = useSiteShipView(project);
  const { view } = ship;
  const back = (
    <Button
      variant="ghost"
      icon="arrow-left"
      tooltip="Closes this and shows the pipeline runner again"
      onClick={onClose}
    >
      Back to the runner
    </Button>
  );
  if (!view) {
    return (
      <main className="grid flex-1 place-items-center" aria-busy={ship.loading}>
        {ship.error ? (
          <EmptyState
            icon="warning-circle"
            title="The Ship settings could not be read"
            body={ship.error}
            action={
              <div className="flex gap-2">
                {back}
                <Button icon="arrow-clockwise" onClick={() => void ship.reload()}>
                  Try again
                </Button>
              </div>
            }
          />
        ) : null}
      </main>
    );
  }
  const { plan, settings, report } = view;
  const shipping = plan.files.filter(f => f.ships).length;
  const held = plan.files.length - shipping;
  const removed = plan.left.filter(f => f.removed).length;
  const subtitle = plan.exported
    ? `What the next Ship changes · ${shipping} written · ${held} held back · ${removed} removed`
    : 'Nothing compressed yet: the plan fills in after the first run';
  return (
    <main className="flex min-h-0 flex-1 gap-5 overflow-y-auto px-6 pt-[18px] pb-[18px]">
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <h1 className="text-screen-title">Ship to {ship.name}&apos;s site</h1>
            <p className="mt-0.5 truncate text-fg-mute">{subtitle}</p>
          </div>
          {back}
        </div>
        {plan.exported ? (
          <SitePlanTable
            files={plan.files}
            left={plan.left}
            fileOf={ship.fileOf}
            editable={!ship.saving}
            onChoice={(asset, choice) => void ship.setChoice(asset, choice)}
            onFile={(asset, file) => void ship.setFile(asset, file)}
          />
        ) : (
          <EmptyState
            icon="rocket-launch"
            title="Nothing compressed yet"
            body="Run the pipeline, or run it with Build only, to see what Ship would write."
          />
        )}
        <SizeReportTable report={report} />
      </div>
      <ShipSettingsPanel
        settings={settings}
        plan={plan}
        repo={ship.repo}
        saving={ship.saving}
        onSettings={ship.saveSettings}
        onTarget={ship.saveTarget}
      />
    </main>
  );
}
