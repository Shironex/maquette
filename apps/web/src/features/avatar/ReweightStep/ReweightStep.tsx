import { Button, Icon, InlineBanner, SegmentedControl } from '@maquette/ui';
import { SheetButton } from '../SheetButton';
import { SheetZoom } from '../SheetZoom';
import { BRIDGES, useReweightStep } from './ReweightStep.hooks';

const POSES = ['At rest', 'Arms level', 'Arms raised'];

/**
 * The first rest pose step when cloth or hair hangs on the arms: a build
 * moves its weight from the arm bones to the trunk on a copy, and the
 * person approves the new weights on their own sheets before the arms are
 * lifted.
 */
export default function ReweightStep() {
  const p = useReweightStep();
  if (!p.show) return null;
  const { reweighted, state } = p;
  return (
    <section aria-labelledby="avatar-reweight-title" className="flex flex-col gap-3">
      <div className="flex items-baseline gap-2">
        <span className="text-caption tracking-[0.04em] text-fg-mute uppercase">Step 1</span>
        <h3 id="avatar-reweight-title" className="text-emphasis font-semibold">
          Free cloth and hair from the arms
        </h3>
      </div>
      {!reweighted ? (
        <p className="text-meta text-fg-mute">
          {p.finding ? `${p.finding.title}. ` : null}A build first moves their weight to the trunk
          on a copy, headless in Blender, keeping the arms, sleeves and gloves, then shows the
          before and after sheets here for your yes. The T-pose comes after.
        </p>
      ) : (
        <>
          {state === 'pending' ? (
            <section
              aria-label="Cloth and hair approval"
              className="flex flex-col gap-3 rounded-card border border-amber/40 bg-amber-soft/40 p-3.5"
            >
              <div className="flex items-start gap-3">
                <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
                  <Icon name="person-arms-spread" size={17} />
                </span>
                <div className="min-w-0 flex-1">
                  <h4 className="text-emphasis leading-5 font-semibold">
                    Approve the new weights?
                  </h4>
                  <p className="mt-0.5 text-meta text-fg-mute">
                    On the after sheet a cape and hair should hang still as the arms lift, and the
                    sleeves should follow the arms without pinching at the shoulder. Cloth sewn to a
                    sleeve still stretches between the arm and the body.
                  </p>
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  variant="secondary"
                  disabled={p.busy}
                  tooltip="Lifts the arms from the model as it is"
                  onClick={p.deny}
                >
                  Deny
                </Button>
                <Button
                  variant="primary"
                  icon="check"
                  loading={p.deciding}
                  disabled={p.busy}
                  tooltip="Approves these weights and normalises the rest pose from them"
                  onClick={p.approve}
                >
                  Approve and go on
                </Button>
              </div>
            </section>
          ) : state === 'denied' ? (
            <InlineBanner
              tone="warning"
              title="You turned the new weights down"
              detail="The arms are lifted from the model as it is. Build again for the rest pose."
            />
          ) : (
            <p className="flex items-center gap-1.5 text-meta text-sage">
              <Icon name="check-circle" size={15} />
              Approved {p.approvedAgo}. The rest pose is lifted from these weights.
            </p>
          )}
          <p className="text-meta text-fg-dim">{p.summary}</p>
          {p.showBridges ? (
            <div className="flex flex-col gap-1.5">
              <SegmentedControl
                label="Faces fused to the body"
                tone="clay"
                options={BRIDGES}
                value={p.bridges}
                disabled={p.busy}
                onChange={p.setBridges}
              />
              <p className="text-meta text-fg-mute">
                {p.bridgesHint} Changing it reweights again, for a new yes.
              </p>
            </div>
          ) : null}
          <div className="flex gap-4">
            <SheetButton
              src={p.sheets?.before ?? null}
              label="Before"
              onOpen={() => p.openZoom('before')}
            />
            <SheetButton
              src={p.sheets?.after ?? null}
              label="After"
              onOpen={() => p.openZoom('after')}
            />
          </div>
          <Button
            variant="secondary"
            size="sm"
            icon="magnifying-glass"
            className="self-start"
            tooltip="The shoulders before and after, side by side and large"
            onClick={() => p.openZoom('shoulders')}
          >
            Compare the shoulders
          </Button>
        </>
      )}
      {p.zoom ? (
        <SheetZoom
          before={p.sheets?.before ?? null}
          after={p.sheets?.after ?? null}
          mode={p.zoom}
          onClose={p.closeZoom}
          title="Freed arms sheets"
          poses={POSES}
        />
      ) : null}
    </section>
  );
}
