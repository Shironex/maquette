import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button, EmptyState } from '@maquette/ui';
import type { ViewId } from '@/state/views';

interface ViewBoundaryProps {
  /** Picking another view clears the failure. */
  view: ViewId;
  children: ReactNode;
}

interface ViewBoundaryState {
  error: Error | null;
  view: ViewId;
}

/** Says so when a view's code fails to load or render, instead of a blank window. */
export class ViewBoundary extends Component<ViewBoundaryProps, ViewBoundaryState> {
  state: ViewBoundaryState = { error: null, view: this.props.view };

  static getDerivedStateFromError(error: Error): Partial<ViewBoundaryState> {
    return { error };
  }

  static getDerivedStateFromProps(
    props: ViewBoundaryProps,
    state: ViewBoundaryState
  ): Partial<ViewBoundaryState> | null {
    return props.view === state.view ? null : { error: null, view: props.view };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('The view failed', error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <main className="grid flex-1 place-items-center p-6">
        <EmptyState
          icon="warning-circle"
          title="This view could not open"
          body={error.message}
          action={
            <Button
              icon="arrow-clockwise"
              tooltip="Loads the app again"
              onClick={() => window.location.reload()}
            >
              Reload
            </Button>
          }
        />
      </main>
    );
  }
}
