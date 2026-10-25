import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Step } from '.';

describe('a handoff step', () => {
  it('numbers the step and shows its title and body', () => {
    render(
      <ol>
        <Step n={2} title="Upload the images">
          <p>Drag them in</p>
        </Step>
      </ol>
    );
    const step = screen.getByRole('listitem');
    expect(step).toHaveTextContent(/^2Upload the imagesDrag them in$/);
  });
});
