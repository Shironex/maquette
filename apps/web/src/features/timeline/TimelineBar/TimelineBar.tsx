import { Button, Icon, IconButton, SegmentedControl } from '@maquette/ui';
import { Menu } from '@/features/common/Menu';
import type { TimelineUnits } from '@/state/timeline';
import { useTimelineBar } from './TimelineBar.hooks';

/** The timeline's toolbar: go to the top, play the scroll, where the page is, add a track. */
export default function TimelineBar() {
  const bar = useTimelineBar();
  const { playhead } = bar;
  const units = [
    { value: 'percent' as const, label: 'Scroll %' },
    {
      value: 'pixels' as const,
      label: 'Pixels',
      disabled: !bar.canPixels,
      hint: bar.canPixels ? undefined : 'Pixels show once the page answers with its scroll',
    },
    { value: 'stops' as const, label: 'Sections' },
  ];
  return (
    <div
      role="toolbar"
      aria-label="Scroll playback"
      className="flex items-center gap-2 border-b border-line px-2.5 py-2"
    >
      <IconButton icon="skip-back" label="Go to the top" size="sm" onClick={playhead.top} />
      <IconButton
        icon={playhead.playing ? 'pause' : 'play'}
        label={playhead.playing ? 'Pause' : 'Play the scroll at reading speed'}
        size="sm"
        shortcut="Space"
        disabled={playhead.count < 2}
        onClick={playhead.togglePlay}
      />
      <Button
        size="sm"
        disabled
        disabledReason="Not built yet: Play spends about 4 seconds on each stop"
      >
        Reading speed
        <Icon name="caret-down" size={12} className="text-fg-mute" />
      </Button>
      <span className="ml-1.5 font-mono text-[12px] text-fg" aria-live="off">
        {bar.percent} <span className="text-fg-mute">· {bar.detail}</span>
      </span>
      <span className="flex-1" />
      <div className="relative">
        <Button
          size="sm"
          icon="plus"
          aria-expanded={bar.menu}
          onClick={() => bar.setMenu(!bar.menu)}
        >
          Add track
          <Icon name="caret-down" size={12} className="text-fg-mute" />
        </Button>
        {bar.menu ? (
          <Menu
            label="Add a track"
            items={bar.items}
            onClose={() => bar.setMenu(false)}
            className="absolute top-full right-0 z-20 mt-1 w-[240px]"
          />
        ) : null}
      </div>
      <SegmentedControl<TimelineUnits>
        label="Units"
        options={units}
        value={bar.units}
        onChange={bar.setUnits}
      />
    </div>
  );
}
