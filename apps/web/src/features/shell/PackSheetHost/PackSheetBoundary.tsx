import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';

interface PackSheetBoundaryProps {
  onClose: () => void;
  children: ReactNode;
}

/**
 * Says so when the pack sheet's code fails to load or render, in a sheet of
 * its own, so the shell and any approval waiting stay on screen. A failed
 * load is remembered until the app loads again, hence Reload.
 */
export default class PackSheetBoundary extends Component<
  PackSheetBoundaryProps,
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('The pack sheet failed', error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <Sheet
        id="export-pack-failed"
        icon="warning-circle"
        title="Export pack could not open"
        onClose={this.props.onClose}
      >
        <div className="flex flex-col items-start gap-3 p-5">
          <p className="text-fg-dim">{error.message}</p>
          <p className="text-fg-mute">Nothing was exported. Loading the app again usually helps.</p>
          <Button
            icon="arrow-clockwise"
            tooltip="Loads the app again"
            onClick={() => window.location.reload()}
          >
            Reload
          </Button>
        </div>
      </Sheet>
    );
  }
}
