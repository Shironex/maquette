import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Group } from '.';

describe('a preset group', () => {
  it('shows its title over its fields', () => {
    const { container } = render(
      <Group title="Rig">
        <p>fields</p>
      </Group>
    );
    expect(container).toHaveTextContent(/^Rigfields$/);
    expect(screen.getByText('fields')).toBeInTheDocument();
  });
});
