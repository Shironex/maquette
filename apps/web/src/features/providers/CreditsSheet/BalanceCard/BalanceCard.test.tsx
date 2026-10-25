import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BalanceCard } from '.';

describe('a balance card', () => {
  it('shows the credits left and what running tasks hold', () => {
    render(
      <BalanceCard
        service="tripo"
        checking={false}
        balance={{
          service: 'tripo',
          key: true,
          credits: 1240,
          held: 30,
          error: null,
          checkedAtMs: 1,
        }}
      />
    );
    const card = screen.getByRole('group', { name: 'Tripo balance' });
    expect(card).toHaveTextContent('1,240');
    expect(card).toHaveTextContent('30 held by running tasks');
  });

  it('says when there is no key to ask with', () => {
    render(
      <BalanceCard
        service="meshy"
        checking={false}
        balance={{
          service: 'meshy',
          key: false,
          credits: null,
          held: null,
          error: null,
          checkedAtMs: 1,
        }}
      />
    );
    expect(screen.getByRole('group', { name: 'Meshy balance' })).toHaveTextContent(
      'No keyAdd one in Settings > 3D providers'
    );
  });
});
