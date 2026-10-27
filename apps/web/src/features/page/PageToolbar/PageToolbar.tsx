import {
  Button,
  Icon,
  SegmentedControl,
  TextField,
  Tooltip,
  cn,
  type IconName,
} from '@maquette/ui';
import { MODES, usePageToolbar } from './PageToolbar.hooks';

interface ToggleChipProps {
  icon: IconName;
  label: string;
  pressed: boolean;
  onClick: () => void;
  hint: string;
}

function ToggleChip({ icon, label, pressed, onClick, hint }: ToggleChipProps) {
  return (
    <Tooltip label={hint}>
      <button
        type="button"
        aria-pressed={pressed}
        onClick={onClick}
        className={cn(
          'inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-control border px-2.5 text-[12px] font-medium whitespace-nowrap transition-colors duration-120',
          pressed
            ? 'border-line-strong bg-hover text-fg [&>svg]:text-clay-text'
            : 'border-line bg-transparent text-fg-dim hover:text-fg'
        )}
      >
        <Icon name={icon} size={14} />
        {label}
      </button>
    </Tooltip>
  );
}

/** The page's title and path, the device width, and the overlays or the compare layout. */
export default function PageToolbar() {
  const t = usePageToolbar();
  return (
    <div className="flex min-w-0 items-center gap-2.5">
      <h1 className="m-0 text-panel-title whitespace-nowrap">{t.title}</h1>
      {t.concept ? null : t.editing ? (
        <form
          className="flex items-center gap-1.5"
          onSubmit={e => {
            e.preventDefault();
            void t.savePath();
          }}
        >
          <TextField
            aria-label="Path on the site"
            value={t.draft}
            autoFocus
            onChange={e => t.setDraft(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Escape') t.cancelPath();
            }}
            mono
            className="h-[26px] w-[180px]"
          />
          <Button size="sm" type="submit" tooltip="Shows this path of the site in the frames">
            Show
          </Button>
        </form>
      ) : (
        <Button
          size="sm"
          iconRight="caret-down"
          aria-label={`Path on the site, ${t.pathLabel}`}
          tooltip="Show another path on the site"
          onClick={t.editPath}
        >
          {t.pathLabel}
        </Button>
      )}
      <span className="flex-1" />
      {t.devicesTab ? null : (
        <SegmentedControl
          label="Device"
          options={t.devices}
          value={t.device}
          onChange={t.setDevice}
        />
      )}
      <span aria-hidden="true" className="h-5 w-px bg-line" />
      {t.mode === 'one' || t.devicesTab ? (
        <>
          <ToggleChip
            icon="map-pin"
            label="Stops"
            pressed={t.showStops}
            onClick={t.toggleStops}
            hint="Shows the current camera stop on the page"
          />
          <ToggleChip
            icon="image"
            label="Poster"
            pressed={t.poster}
            onClick={t.togglePoster}
            hint="Shows the page as it is without WebGL"
          />
        </>
      ) : (
        <SegmentedControl label="Compare" options={MODES} value={t.mode} onChange={t.setMode} />
      )}
    </div>
  );
}
