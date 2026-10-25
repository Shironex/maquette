import { Component, Fragment, type ErrorInfo, type ReactNode } from 'react';
import { Button, EmptyState } from '@maquette/ui';

interface CanvasBoundaryProps {
  children: ReactNode;
  /** A new value (the model shown, say) gives the canvas a fresh try. */
  resetKey?: string | null;
}

interface CanvasBoundaryState {
  error: Error | null;
  tries: number;
  resetKey: string | null | undefined;
}

/**
 * Keeps a 3D canvas that throws (a lazy chunk that failed, a renderer that
 * could not start) from blanking the window: a small message with Reload
 * takes its place, and Reload mounts the canvas again.
 */
export default class CanvasBoundary extends Component<CanvasBoundaryProps, CanvasBoundaryState> {
  state: CanvasBoundaryState = { error: null, tries: 0, resetKey: this.props.resetKey };

  static getDerivedStateFromError(error: Error): Partial<CanvasBoundaryState> {
    return { error };
  }

  static getDerivedStateFromProps(
    props: CanvasBoundaryProps,
    state: CanvasBoundaryState
  ): Partial<CanvasBoundaryState> | null {
    return props.resetKey === state.resetKey ? null : { error: null, resetKey: props.resetKey };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('The 3D preview failed', error, info.componentStack);
  }

  render() {
    const { error, tries } = this.state;
    if (!error) return <Fragment key={tries}>{this.props.children}</Fragment>;
    return (
      <div role="alert" className="absolute inset-0 grid place-items-center p-6">
        <EmptyState
          icon="warning-circle"
          title="The 3D preview stopped"
          body={error.message || 'Something went wrong while drawing the model.'}
          action={
            <Button
              icon="arrow-clockwise"
              onClick={() => this.setState(s => ({ error: null, tries: s.tries + 1 }))}
            >
              Reload
            </Button>
          }
        />
      </div>
    );
  }
}
